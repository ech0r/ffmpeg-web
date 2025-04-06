#include <emscripten.h>
#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>
#include <libswscale/swscale.h>
#include <libavutil/imgutils.h>
#include <libavutil/opt.h>
#include <libavutil/error.h>
#include <libswresample/swresample.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Structure to hold transcoding progress and results
typedef struct {
    int success;
    int progress;
    char error_message[256];
    uint8_t* output_data;
    size_t output_size;
} TranscodeResult;

// Custom callback to update progress
typedef void (*ProgressCallback)(int progress);
static ProgressCallback progress_callback = NULL;

// Global result structure
static TranscodeResult* global_result = NULL;

// Initialize FFmpeg libraries
EMSCRIPTEN_KEEPALIVE
void init_ffmpeg() {
#if LIBAVCODEC_VERSION_INT < AV_VERSION_INT(58, 9, 100)
    av_register_all();
#endif
    av_log_set_level(AV_LOG_INFO);
}

// Custom AVIOContext write callback
static int write_packet(void *opaque, uint8_t *buf, int buf_size) {
    TranscodeResult *result = (TranscodeResult*)opaque;
    
    // Reallocate output buffer to accommodate new data
    uint8_t* new_buffer = realloc(result->output_data, result->output_size + buf_size);
    if (!new_buffer) {
        return AVERROR(ENOMEM); // Memory allocation error
    }
    
    // Copy new data to the extended buffer
    memcpy(new_buffer + result->output_size, buf, buf_size);
    result->output_data = new_buffer;
    result->output_size += buf_size;
    
    return buf_size;
}

// Set progress callback
EMSCRIPTEN_KEEPALIVE
void set_progress_callback(ProgressCallback callback) {
    progress_callback = callback;
}

// Helper function to update progress
static void update_progress(int progress) {
    if (global_result) {
        global_result->progress = progress;
    }
    
    if (progress_callback) {
        progress_callback(progress);
    }
}

// Process a single packet
static int process_packet(
    AVFormatContext *output_fmt_ctx,
    AVCodecContext *enc_ctx,
    AVFrame *frame,
    AVPacket *pkt,
    int stream_index
) {
    int ret;

    // Send frame to encoder
    ret = avcodec_send_frame(enc_ctx, frame);
    if (ret < 0) {
        return ret;
    }

    // Get all available packets
    while (ret >= 0) {
        ret = avcodec_receive_packet(enc_ctx, pkt);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            return 0;
        } else if (ret < 0) {
            return ret;
        }

        // Prepare packet for muxing
        pkt->stream_index = stream_index;
        av_packet_rescale_ts(pkt, enc_ctx->time_base, output_fmt_ctx->streams[stream_index]->time_base);

        // Write packet to output
        ret = av_interleaved_write_frame(output_fmt_ctx, pkt);
        if (ret < 0) {
            return ret;
        }
    }

    return 0;
}

// Main transcoding function
EMSCRIPTEN_KEEPALIVE
TranscodeResult* transcode(
    uint8_t* input_data, 
    size_t input_size,
    const char* output_format,
    const char* video_codec_name,
    const char* audio_codec_name,
    int video_bitrate,
    int audio_bitrate,
    const char* resolution
) {
    AVFormatContext *input_ctx = NULL;
    AVFormatContext *output_ctx = NULL;
    AVIOContext *input_io_ctx = NULL;
    AVIOContext *output_io_ctx = NULL;
    AVCodecContext *video_dec_ctx = NULL;
    AVCodecContext *audio_dec_ctx = NULL;
    AVCodecContext *video_enc_ctx = NULL;
    AVCodecContext *audio_enc_ctx = NULL;
    const AVCodec *video_decoder = NULL;
    const AVCodec *audio_decoder = NULL;
    const AVCodec *video_encoder = NULL;
    const AVCodec *audio_encoder = NULL;
    AVFrame *video_frame = NULL;
    AVFrame *audio_frame = NULL;
    AVPacket *pkt = NULL;
    SwsContext *sws_ctx = NULL;
    SwrContext *swr_ctx = NULL;
    int video_stream_idx = -1;
    int audio_stream_idx = -1;
    int out_video_stream_idx = -1;
    int out_audio_stream_idx = -1;
    unsigned char *input_io_buffer = NULL;
    unsigned char *output_io_buffer = NULL;
    int ret = 0;
    int64_t total_frames = 0;
    int64_t processed_frames = 0;
    
    // Allocate result structure
    TranscodeResult *result = calloc(1, sizeof(TranscodeResult));
    if (!result) {
        fprintf(stderr, "Failed to allocate result structure\n");
        return NULL;
    }
    
    // Set global result for progress tracking
    global_result = result;
    
    // Initialize result
    result->success = 0;
    result->progress = 0;
    result->output_data = NULL;
    result->output_size = 0;
    snprintf(result->error_message, sizeof(result->error_message), "");
    
    // Create input IO context from memory buffer
    input_io_buffer = av_malloc(input_size + AV_INPUT_BUFFER_PADDING_SIZE);
    if (!input_io_buffer) {
        snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate input buffer");
        goto cleanup;
    }
    
    memcpy(input_io_buffer, input_data, input_size);
    memset(input_io_buffer + input_size, 0, AV_INPUT_BUFFER_PADDING_SIZE);
    
    input_io_ctx = avio_alloc_context(
        input_io_buffer,
        input_size,
        0,
        NULL,
        NULL,  // No read callback needed, using buffer directly
        NULL,  // No write callback needed for input
        NULL   // No seek callback needed for this simple case
    );
    
    if (!input_io_ctx) {
        snprintf(result->error_message, sizeof(result->error_message), "Failed to create input IO context");
        goto cleanup;
    }
    
    // Allocate input format context
    input_ctx = avformat_alloc_context();
    if (!input_ctx) {
        snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate input context");
        goto cleanup;
    }
    
    input_ctx->pb = input_io_ctx;
    
    // Open input
    ret = avformat_open_input(&input_ctx, NULL, NULL, NULL);
    if (ret < 0) {
        av_strerror(ret, result->error_message, sizeof(result->error_message));
        goto cleanup;
    }
    
    // Find stream info
    ret = avformat_find_stream_info(input_ctx, NULL);
    if (ret < 0) {
        av_strerror(ret, result->error_message, sizeof(result->error_message));
        goto cleanup;
    }
    
    // Create output format context
    ret = avformat_alloc_output_context2(&output_ctx, NULL, output_format, NULL);
    if (ret < 0 || !output_ctx) {
        snprintf(result->error_message, sizeof(result->error_message), "Failed to create output context");
        goto cleanup;
    }
    
    // Create output IO context for memory output
    output_io_buffer = av_malloc(4096);
    if (!output_io_buffer) {
        snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate output buffer");
        goto cleanup;
    }
    
    output_io_ctx = avio_alloc_context(
        output_io_buffer,
        4096,
        1,
        result,  // User data - pass result struct to write callback
        NULL,    // No read callback needed for output
        write_packet,  // Write callback
        NULL     // No seek callback needed for this simple case
    );
    
    if (!output_io_ctx) {
        snprintf(result->error_message, sizeof(result->error_message), "Failed to create output IO context");
        goto cleanup;
    }
    
    output_ctx->pb = output_io_ctx;
    
    // Find video and audio streams
    for (unsigned int i = 0; i < input_ctx->nb_streams; i++) {
        AVStream *in_stream = input_ctx->streams[i];
        
        if (in_stream->codecpar->codec_type == AVMEDIA_TYPE_VIDEO && video_stream_idx < 0) {
            video_stream_idx = i;
            
            // Create output video stream
            AVStream *out_stream = avformat_new_stream(output_ctx, NULL);
            if (!out_stream) {
                snprintf(result->error_message, sizeof(result->error_message), "Failed to create output video stream");
                goto cleanup;
            }
            
            out_video_stream_idx = out_stream->index;
            
            // Find decoder
            video_decoder = avcodec_find_decoder(in_stream->codecpar->codec_id);
            if (!video_decoder) {
                snprintf(result->error_message, sizeof(result->error_message), "Unsupported video codec");
                goto cleanup;
            }
            
            // Allocate decoder context
            video_dec_ctx = avcodec_alloc_context3(video_decoder);
            if (!video_dec_ctx) {
                snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate video decoder context");
                goto cleanup;
            }
            
            // Copy parameters from input stream to decoder context
            ret = avcodec_parameters_to_context(video_dec_ctx, in_stream->codecpar);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            // Open decoder
            ret = avcodec_open2(video_dec_ctx, video_decoder, NULL);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            // Find encoder
            video_encoder = avcodec_find_encoder_by_name(video_codec_name);
            if (!video_encoder) {
                snprintf(result->error_message, sizeof(result->error_message), "Video encoder '%s' not found", video_codec_name);
                goto cleanup;
            }
            
            // Allocate encoder context
            video_enc_ctx = avcodec_alloc_context3(video_encoder);
            if (!video_enc_ctx) {
                snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate video encoder context");
                goto cleanup;
            }
            
            // Set video encoder parameters
            int width = video_dec_ctx->width;
            int height = video_dec_ctx->height;
            
            // Apply custom resolution if needed
            if (resolution && strcmp(resolution, "same") != 0) {
                if (sscanf(resolution, "%dx%d", &width, &height) != 2) {
                    // Try preset resolutions
                    if (strcmp(resolution, "720p") == 0) {
                        width = 1280;
                        height = 720;
                    } else if (strcmp(resolution, "1080p") == 0) {
                        width = 1920;
                        height = 1080;
                    } else if (strcmp(resolution, "480p") == 0) {
                        width = 854;
                        height = 480;
                    } else if (strcmp(resolution, "360p") == 0) {
                        width = 640;
                        height = 360;
                    }
                }
            }
            
            video_enc_ctx->height = height;
            video_enc_ctx->width = width;
            video_enc_ctx->sample_aspect_ratio = video_dec_ctx->sample_aspect_ratio;
            video_enc_ctx->time_base = av_inv_q(input_ctx->streams[video_stream_idx]->r_frame_rate);
            video_enc_ctx->framerate = input_ctx->streams[video_stream_idx]->r_frame_rate;
            video_enc_ctx->gop_size = 25;
            video_enc_ctx->max_b_frames = 3;
            video_enc_ctx->pix_fmt = video_encoder->pix_fmts ? video_encoder->pix_fmts[0] : AV_PIX_FMT_YUV420P;
            video_enc_ctx->bit_rate = video_bitrate * 1000;
            
            if (output_ctx->oformat->flags & AVFMT_GLOBALHEADER) {
                video_enc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
            }
            
            // Open encoder
            ret = avcodec_open2(video_enc_ctx, video_encoder, NULL);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            // Copy parameters to output stream
            ret = avcodec_parameters_from_context(out_stream->codecpar, video_enc_ctx);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            out_stream->time_base = video_enc_ctx->time_base;
            
            // Create scaling context if needed
            if (video_dec_ctx->width != video_enc_ctx->width ||
                video_dec_ctx->height != video_enc_ctx->height ||
                video_dec_ctx->pix_fmt != video_enc_ctx->pix_fmt) {
                
                sws_ctx = sws_getContext(
                    video_dec_ctx->width, video_dec_ctx->height, video_dec_ctx->pix_fmt,
                    video_enc_ctx->width, video_enc_ctx->height, video_enc_ctx->pix_fmt,
                    SWS_BICUBIC, NULL, NULL, NULL
                );
                
                if (!sws_ctx) {
                    snprintf(result->error_message, sizeof(result->error_message), "Failed to create scaling context");
                    goto cleanup;
                }
            }
            
            // Estimate total frames for progress tracking
            double duration_seconds = (double)input_ctx->duration / AV_TIME_BASE;
            double fps = av_q2d(input_ctx->streams[video_stream_idx]->r_frame_rate);
            total_frames = (int64_t)(duration_seconds * fps);
        }
        
        else if (in_stream->codecpar->codec_type == AVMEDIA_TYPE_AUDIO && audio_stream_idx < 0) {
            audio_stream_idx = i;
            
            // Create output audio stream
            AVStream *out_stream = avformat_new_stream(output_ctx, NULL);
            if (!out_stream) {
                snprintf(result->error_message, sizeof(result->error_message), "Failed to create output audio stream");
                goto cleanup;
            }
            
            out_audio_stream_idx = out_stream->index;
            
            // Find decoder
            audio_decoder = avcodec_find_decoder(in_stream->codecpar->codec_id);
            if (!audio_decoder) {
                snprintf(result->error_message, sizeof(result->error_message), "Unsupported audio codec");
                goto cleanup;
            }
            
            // Allocate decoder context
            audio_dec_ctx = avcodec_alloc_context3(audio_decoder);
            if (!audio_dec_ctx) {
                snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate audio decoder context");
                goto cleanup;
            }
            
            // Copy parameters from input stream to decoder context
            ret = avcodec_parameters_to_context(audio_dec_ctx, in_stream->codecpar);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            // Open decoder
            ret = avcodec_open2(audio_dec_ctx, audio_decoder, NULL);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            // Find encoder
            audio_encoder = avcodec_find_encoder_by_name(audio_codec_name);
            if (!audio_encoder) {
                snprintf(result->error_message, sizeof(result->error_message), "Audio encoder '%s' not found", audio_codec_name);
                goto cleanup;
            }
            
            // Allocate encoder context
            audio_enc_ctx = avcodec_alloc_context3(audio_encoder);
            if (!audio_enc_ctx) {
                snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate audio encoder context");
                goto cleanup;
            }
            
            // Set audio encoder parameters
            audio_enc_ctx->channels = audio_dec_ctx->channels;
            audio_enc_ctx->channel_layout = av_get_default_channel_layout(audio_dec_ctx->channels);
            audio_enc_ctx->sample_rate = audio_dec_ctx->sample_rate;
            audio_enc_ctx->sample_fmt = audio_encoder->sample_fmts ? audio_encoder->sample_fmts[0] : AV_SAMPLE_FMT_FLTP;
            audio_enc_ctx->time_base = (AVRational){1, audio_dec_ctx->sample_rate};
            audio_enc_ctx->bit_rate = audio_bitrate * 1000;
            
            if (output_ctx->oformat->flags & AVFMT_GLOBALHEADER) {
                audio_enc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
            }
            
            // Open encoder
            ret = avcodec_open2(audio_enc_ctx, audio_encoder, NULL);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            // Copy parameters to output stream
            ret = avcodec_parameters_from_context(out_stream->codecpar, audio_enc_ctx);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            out_stream->time_base = audio_enc_ctx->time_base;
            
            // Create audio resampling context if needed
            if (audio_dec_ctx->channel_layout != audio_enc_ctx->channel_layout ||
                audio_dec_ctx->sample_rate != audio_enc_ctx->sample_rate ||
                audio_dec_ctx->sample_fmt != audio_enc_ctx->sample_fmt) {
                
                swr_ctx = swr_alloc();
                if (!swr_ctx) {
                    snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate resampling context");
                    goto cleanup;
                }
                
                av_opt_set_int(swr_ctx, "in_channel_count", audio_dec_ctx->channels, 0);
                av_opt_set_int(swr_ctx, "in_sample_rate", audio_dec_ctx->sample_rate, 0);
                av_opt_set_sample_fmt(swr_ctx, "in_sample_fmt", audio_dec_ctx->sample_fmt, 0);
                
                av_opt_set_int(swr_ctx, "out_channel_count", audio_enc_ctx->channels, 0);
                av_opt_set_int(swr_ctx, "out_sample_rate", audio_enc_ctx->sample_rate, 0);
                av_opt_set_sample_fmt(swr_ctx, "out_sample_fmt", audio_enc_ctx->sample_fmt, 0);
                
                if ((ret = swr_init(swr_ctx)) < 0) {
                    av_strerror(ret, result->error_message, sizeof(result->error_message));
                    goto cleanup;
                }
            }
        }
    }
    
    // Check if we have at least one stream to process
    if (video_stream_idx < 0 && audio_stream_idx < 0) {
        snprintf(result->error_message, sizeof(result->error_message), "No audio or video streams found");
        goto cleanup;
    }
    
    // Write output header
    ret = avformat_write_header(output_ctx, NULL);
    if (ret < 0) {
        av_strerror(ret, result->error_message, sizeof(result->error_message));
        goto cleanup;
    }
    
    // Allocate frames and packet
    video_frame = av_frame_alloc();
    audio_frame = av_frame_alloc();
    pkt = av_packet_alloc();
    
    if (!video_frame || !audio_frame || !pkt) {
        snprintf(result->error_message, sizeof(result->error_message), "Failed to allocate frames or packet");
        goto cleanup;
    }
    
    // Main processing loop
    while (1) {
        ret = av_read_frame(input_ctx, pkt);
        
        // End of file
        if (ret == AVERROR_EOF) {
            // Flush decoders and encoders
            if (video_dec_ctx) {
                avcodec_send_packet(video_dec_ctx, NULL);
                while (1) {
                    ret = avcodec_receive_frame(video_dec_ctx, video_frame);
                    if (ret == AVERROR_EOF || ret == AVERROR(EAGAIN)) {
                        break;
                    }
                    
                    // Scale frame if needed
                    if (sws_ctx) {
                        AVFrame *scaled_frame = av_frame_alloc();
                        scaled_frame->format = video_enc_ctx->pix_fmt;
                        scaled_frame->width = video_enc_ctx->width;
                        scaled_frame->height = video_enc_ctx->height;
                        av_frame_get_buffer(scaled_frame, 0);
                        
                        sws_scale(sws_ctx, (const uint8_t* const*)video_frame->data, video_frame->linesize,
                            0, video_dec_ctx->height, scaled_frame->data, scaled_frame->linesize);
                            
                        scaled_frame->pts = av_rescale_q(video_frame->pts, 
                            input_ctx->streams[video_stream_idx]->time_base, video_enc_ctx->time_base);
                            
                        process_packet(output_ctx, video_enc_ctx, scaled_frame, pkt, out_video_stream_idx);
                        av_frame_free(&scaled_frame);
                    } else {
                        video_frame->pts = av_rescale_q(video_frame->pts, 
                            input_ctx->streams[video_stream_idx]->time_base, video_enc_ctx->time_base);
                            
                        process_packet(output_ctx, video_enc_ctx, video_frame, pkt, out_video_stream_idx);
                    }
                }
                
                // Flush video encoder
                process_packet(output_ctx, video_enc_ctx, NULL, pkt, out_video_stream_idx);
            }
            
            if (audio_dec_ctx) {
                avcodec_send_packet(audio_dec_ctx, NULL);
                while (1) {
                    ret = avcodec_receive_frame(audio_dec_ctx, audio_frame);
                    if (ret == AVERROR_EOF || ret == AVERROR(EAGAIN)) {
                        break;
                    }
                    
                    // Resample frame if needed
                    if (swr_ctx) {
                        AVFrame *resampled_frame = av_frame_alloc();
                        resampled_frame->format = audio_enc_ctx->sample_fmt;
                        resampled_frame->channel_layout = audio_enc_ctx->channel_layout;
                        resampled_frame->channels = audio_enc_ctx->channels;
                        resampled_frame->sample_rate = audio_enc_ctx->sample_rate;
                        resampled_frame->nb_samples = audio_frame->nb_samples;
                        av_frame_get_buffer(resampled_frame, 0);
                        
                        swr_convert(swr_ctx, resampled_frame->data, resampled_frame->nb_samples,
                            (const uint8_t**)audio_frame->data, audio_frame->nb_samples);
                            
                        resampled_frame->pts = av_rescale_q(audio_frame->pts, 
                            input_ctx->streams[audio_stream_idx]->time_base, audio_enc_ctx->time_base);
                            
                        process_packet(output_ctx, audio_enc_ctx, resampled_frame, pkt, out_audio_stream_idx);
                        av_frame_free(&resampled_frame);
                    } else {
                        audio_frame->pts = av_rescale_q(audio_frame->pts, 
                            input_ctx->streams[audio_stream_idx]->time_base, audio_enc_ctx->time_base);
                            
                        process_packet(output_ctx, audio_enc_ctx, audio_frame, pkt, out_audio_stream_idx);
                    }
                }
                
                // Flush audio encoder
                process_packet(output_ctx, audio_enc_ctx, NULL, pkt, out_audio_stream_idx);
            }
            
            break;
        } else if (ret < 0) {
            av_strerror(ret, result->error_message, sizeof(result->error_message));
            goto cleanup;
        }
        
        // Process video
        if (pkt->stream_index == video_stream_idx) {
            ret = avcodec_send_packet(video_dec_ctx, pkt);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            while (ret >= 0) {
                ret = avcodec_receive_frame(video_dec_ctx, video_frame);
                if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
                    break;
                } else if (ret < 0) {
                    av_strerror(ret, result->error_message, sizeof(result->error_message));
                    goto cleanup;
                }
                
                processed_frames++;
                if (total_frames > 0) {
                    update_progress((int)((processed_frames * 100) / total_frames));
                }
                
                // Scale frame if needed
                if (sws_ctx) {
                    AVFrame *scaled_frame = av_frame_alloc();
                    scaled_frame->format = video_enc_ctx->pix_fmt;
                    scaled_frame->width = video_enc_ctx->width;
                    scaled_frame->height = video_enc_ctx->height;
                    av_frame_get_buffer(scaled_frame, 0);
                    
                    sws_scale(sws_ctx, (const uint8_t* const*)video_frame->data, video_frame->linesize,
                        0, video_dec_ctx->height, scaled_frame->data, scaled_frame->linesize);
                        
                    scaled_frame->pts = av_rescale_q(video_frame->pts, 
                        input_ctx->streams[video_stream_idx]->time_base, video_enc_ctx->time_base);
                        
                    process_packet(output_ctx, video_enc_ctx, scaled_frame, pkt, out_video_stream_idx);
                    av_frame_free(&scaled_frame);
                } else {
                    video_frame->pts = av_rescale_q(video_frame->pts, 
                        input_ctx->streams[video_stream_idx]->time_base, video_enc_ctx->time_base);
                        
                    process_packet(output_ctx, video_enc_ctx, video_frame, pkt, out_video_stream_idx);
                }
            }
        }
        
        // Process audio
        else if (pkt->stream_index == audio_stream_idx) {
            ret = avcodec_send_packet(audio_dec_ctx, pkt);
            if (ret < 0) {
                av_strerror(ret, result->error_message, sizeof(result->error_message));
                goto cleanup;
            }
            
            while (ret >= 0) {
                ret = avcodec_receive_frame(audio_dec_ctx, audio_frame);
                if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
                    break;
                } else if (ret < 0) {
                    av_strerror(ret, result->error_message, sizeof(result->error_message));
                    goto cleanup;
                }
                
                // Resample frame if needed
                if (swr_ctx) {
                    AVFrame *resampled_frame = av_frame_alloc();
                    resampled_frame->format = audio_enc_ctx->sample_fmt;
                    resampled_frame->channel_layout = audio_enc_ctx->channel_layout;
                    resampled_frame->channels = audio_enc_ctx->channels;
                    resampled_frame->sample_rate = audio_enc_ctx->sample_rate;
                    resampled_frame->nb_samples = audio_enc_ctx->sample_rate * audio_frame->nb_samples / audio_dec_ctx->sample_rate;
                    av_frame_get_buffer(resampled_frame, 0);
                    
                    swr_convert(swr_ctx, resampled_frame->data, resampled_frame->nb_samples,
                        (const uint8_t**)audio_frame->data, audio_frame->nb_samples);
                        
                    resampled_frame->pts = av_rescale_q(audio_frame->pts, 
                        input_ctx->streams[audio_stream_idx]->time_base, audio_enc_ctx->time_base);
                        
                    process_packet(output_ctx, audio_enc_ctx, resampled_frame, pkt, out_audio_stream_idx);
                    av_frame_free(&resampled_frame);
                } else {
                    audio_frame->pts = av_rescale_q(audio_frame->pts, 
                        input_ctx->streams[audio_stream_idx]->time_base, audio_enc_ctx->time_base);
                        
                    process_packet(output_ctx, audio_enc_ctx, audio_frame, pkt, out_audio_stream_idx);
                }
            }
        }
        
        av_packet_unref(pkt);
    }
    
    // Write trailer
    ret = av_write_trailer(output_ctx);
    if (ret < 0) {
        av_strerror(ret, result->error_message, sizeof(result->error_message));
        goto cleanup;
    }
    
    // Set final progress
    update_progress(100);
    
    // Success
    result->success = 1;
    
cleanup:
    // Clean up resources
    if (video_frame) av_frame_free(&video_frame);
    if (audio_frame) av_frame_free(&audio_frame);
    if (pkt) av_packet_free(&pkt);
    
    if (video_dec_ctx) avcodec_free_context(&video_dec_ctx);
    if (audio_dec_ctx) avcodec_free_context(&audio_dec_ctx);
    if (video_enc_ctx) avcodec_free_context(&video_enc_ctx);
    if (audio_enc_ctx) avcodec_free_context(&audio_enc_ctx);
    
    if (sws_ctx) sws_freeContext(sws_ctx);
    if (swr_ctx) swr_free(&swr_ctx);
    
    if (input_io_buffer) av_free(input_io_buffer);
    if (output_io_buffer) av_free(output_io_buffer);
    
    if (input_io_ctx) {
        input_io_ctx->buffer = NULL; // Prevent double-free since buffer is freed separately
        avio_context_free(&input_io_ctx);
    }
    
    if (output_io_ctx) {
        output_io_ctx->buffer = NULL; // Prevent double-free
        avio_context_free(&output_io_ctx);
    }
    
    if (input_ctx) avformat_close_input(&input_ctx);
    if (output_ctx) avformat_free_context(output_ctx);
    
    // Clear global result
    global_result = NULL;
    
    return result;
}

// Free resources associated with a TranscodeResult
EMSCRIPTEN_KEEPALIVE
void free_transcode_result(TranscodeResult* result) {
    if (result) {
        if (result->output_data) {
            free(result->output_data);
        }
        free(result);
    }
}
