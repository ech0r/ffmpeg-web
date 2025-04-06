// Pre-js file for FFmpeg WebAssembly module
// This is prepended to the generated JavaScript wrapper

// Store the Module object
var FFmpegModule = Module;

// Progress callback function
var progressCallback = null;

// Set up progress callback accessor
FFmpegModule['setProgressCallback'] = function(callback) {
  progressCallback = callback;
};

// Function to call the progress callback
FFmpegModule['_update_progress_js'] = function(progress) {
  if (typeof progressCallback === 'function') {
    progressCallback(progress);
  }
};

// Asynchronous transcode function using ccall
FFmpegModule['transcodeAsync'] = function(inputData, outputFormat, videoCodec, audioCodec, videoBitrate, audioBitrate, resolution) {
  return new Promise(function(resolve, reject) {
    // Create Uint8Array from input data
    var dataPtr = FFmpegModule._malloc(inputData.length);
    FFmpegModule.HEAPU8.set(inputData, dataPtr);
    
    // Set up progress callback
    var lastProgress = 0;
    FFmpegModule.setProgressCallback(function(progress) {
      if (progress > lastProgress) {
        lastProgress = progress;
        if (typeof Module['onProgress'] === 'function') {
          Module['onProgress'](progress);
        }
      }
    });
    
    // Call the C transcode function
    var resultPtr = FFmpegModule.ccall(
      'transcode',
      'number',
      ['number', 'number', 'string', 'string', 'string', 'number', 'number', 'string'],
      [dataPtr, inputData.length, outputFormat, videoCodec, audioCodec, videoBitrate, audioBitrate, resolution]
    );
    
    // Free the input data memory
    FFmpegModule._free(dataPtr);
    
    // Check if the transcode was successful
    if (resultPtr === 0) {
      reject(new Error('Transcoding failed: No result returned'));
      return;
    }
    
    // Extract result data
    var success = FFmpegModule.getValue(resultPtr, 'i32');
    
    if (success) {
      // Extract output data
      var outputDataPtr = FFmpegModule.getValue(resultPtr + 12, '*');
      var outputSize = FFmpegModule.getValue(resultPtr + 16, 'i64');
      
      // Create output buffer
      var outputData = new Uint8Array(outputSize);
      outputData.set(FFmpegModule.HEAPU8.subarray(outputDataPtr, outputDataPtr + outputSize));
      
      // Free the result struct
      FFmpegModule.ccall('free_transcode_result', null, ['number'], [resultPtr]);
      
      resolve(outputData);
    } else {
      // Get error message
      var errorMsgPtr = resultPtr + 8;
      var errorMsg = FFmpegModule.UTF8ToString(errorMsgPtr);
      
      // Free the result struct
      FFmpegModule.ccall('free_transcode_result', null, ['number'], [resultPtr]);
      
      reject(new Error('Transcoding failed: ' + errorMsg));
    }
  });
};
