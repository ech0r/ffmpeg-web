// ffmpeg-bridge.js
let ffmpeg = null;
let progress_callback = null;
let logger_callback = null;

export function createFFmpeg() {
  if (!ffmpeg) {
    ffmpeg = FFmpeg.createFFmpeg({
      log: true,
      progress: ({ ratio }) => {
        if (progress_callback) {
          progress_callback(ratio * 100);
        }
      },
    });
  }
  return ffmpeg;
}

export function setProgressCallback(callback) {
  progress_callback = callback;
}

export function setLoggerCallback(callback) {
  logger_callback = callback;
  if (ffmpeg) {
    ffmpeg.setLogger(({ message }) => {
      if (logger_callback) {
        logger_callback(message);
      }
    });
  }
}

export async function loadFFmpeg() {
  const instance = createFFmpeg();
  if (!instance.isLoaded()) {
    await instance.load();
  }
  return true;
}

export async function runFFmpeg(args) {
  const instance = createFFmpeg();
  await instance.run(...args);
  return true;
}

export function writeFile(name, data) {
  const instance = createFFmpeg();
  instance.FS('writeFile', name, data);
}

export function readFile(name) {
  const instance = createFFmpeg();
  return instance.FS('readFile', name);
}
