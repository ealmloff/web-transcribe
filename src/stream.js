/**
 * Streams audio from tab/window capture and calls the callback with raw audio samples
 * @param {Function} callback - Called with { samples, sampleRate } on each audio frame
 * @param {Object} options - Optional configuration { bufferSize: 4096 }
 * @returns {Promise<Function>} Returns a stop function to end the stream
 */
export async function streamMicrophone(callback, options = {}) {
  const bufferSize = options.bufferSize || 4096;
  const from_display = options.fromDisplay || false;

  // Request microphone access
  let stream;
  if (from_display) {
    stream = await navigator.mediaDevices.getDisplayMedia({
      audio: true,
    });
  } else {
    stream = await navigator.mediaDevices.getUserMedia({
      audio: {},
    });
  }

  // Create audio context
  const audioContext = new (window.AudioContext || window.webkitAudioContext)();
  const sampleRate = audioContext.sampleRate;

  // Create the worklet processor code as a blob
  const workletCode = `
    class AudioCaptureProcessor extends AudioWorkletProcessor {
      constructor(options) {
        super();
        this.bufferSize = options.processorOptions.bufferSize;
        this.buffer = [];
        this.sampleRate = ${sampleRate};
      }

      process(inputs, outputs, parameters) {
        const input = inputs[0];
        if (input.length > 0) {
          const samples = input[0]; // Get channel 0

          // Accumulate samples
          this.buffer.push(...samples);

          // When we have enough samples, send them
          if (this.buffer.length >= this.bufferSize) {
            this.port.postMessage({
              samples: this.buffer.slice(0, this.bufferSize),
              sampleRate: this.sampleRate
            });

            // Keep overflow samples for next chunk
            this.buffer = this.buffer.slice(this.bufferSize);
          }
        }

        return true; // Keep processor alive
      }
    }

    registerProcessor('audio-capture-processor', AudioCaptureProcessor);
  `;

  // Create blob URL for the worklet
  const blob = new Blob([workletCode], { type: "application/javascript" });
  const workletUrl = URL.createObjectURL(blob);

  // Load the worklet module
  await audioContext.audioWorklet.addModule(workletUrl);

  // Create the worklet node
  const workletNode = new AudioWorkletNode(
    audioContext,
    "audio-capture-processor",
    {
      processorOptions: { bufferSize },
    },
  );

  // Handle messages from the worklet
  workletNode.port.onmessage = (event) => {
    callback({
      samples: new Float32Array(event.data.samples),
      sampleRate: event.data.sampleRate,
      bufferLength: event.data.samples.length,
    });
  };

  // Connect captured audio stream to worklet
  const source = audioContext.createMediaStreamSource(stream);
  source.connect(workletNode);
  workletNode.connect(audioContext.destination);

  // Return stop function
  return function stop() {
    if (workletNode) {
      workletNode.disconnect();
    }
    if (source) {
      source.disconnect();
    }
    if (audioContext) {
      audioContext.close();
    }
    // Clean up blob URL
    URL.revokeObjectURL(workletUrl);
    // Stop all tracks in the stream
    stream.getTracks().forEach((track) => track.stop());
  };
}
