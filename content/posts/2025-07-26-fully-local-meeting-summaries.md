+++
title = "Fully Local Meeting Summaries"
date = "2025-07-26"

[taxonomies]
tags=["coding", "ai"]
+++

Remote work has transformed how we conduct meetings, but it's also created a new challenge: keeping track of everything discussed across countless video calls. While most platforms offer cloud recording, searching through hours of audio for that one key decision made three meetings ago remains a painful experience. This led me to build [**Essence**](https://github.com/orellazri/essence), a fully local command-line tool that transcribes meeting audio and generates structured summaries without sending any data to external services.

This project builds on my previous exploration of local AI with Rust - [RAG Pipeline to Chat with My Obsidian Vault](https://orellazri.com/posts/rag-pipeline-chat-with-my-obsidian-vault/), where I first dove into running AI models locally for privacy and performance.

## The Problem

Meeting recordings pile up quickly, but finding specific information requires listening through entire recordings. What I needed was something that could:

1. Convert audio recordings to searchable text
2. Generate concise summaries highlighting key decisions and action items
3. Work entirely offline for privacy and speed
4. Integrate seamlessly into my existing CLI-based workflow

I did not want to use/pay for any cloud services that join the meetings for me (\*\*_cough_\*\* [Some Stupid Name] The Note Taker \*\*_cough_\*\*), and I wanted to use my own hardware.

## Technical Architecture

Essence is built in Rust and follows a modular design with two primary components: transcription and summarization. I'm using OpenAI's Whisper model running locally via the `whisper-rs` crate for speech recognition, and Ollama for local language model inference. Everything runs entirely on my machine - no data ever leaves my system, which you know I love.

I achieved good results by using Apple's Metal acceleration for the Whisper model (`ggml-large-v3.bin`), and the `gemma3:27b` model from Ollama for summarization. On my M4 Pro, transcribing a and summarizing a 45-minute meeting took less than 8 minutes (your mileage may vary).

### Transcription

The transcription module wraps Whisper's C++ implementation, handling the complex audio preprocessing required for speech recognition.

We first have to read the `wav` audio file (using `hound`) into a vector of 16-bit integers.

```rust
let samples: Vec<i16> = WavReader::open(audio_path)
    .map_err(|e| Error::new(&format!("Failed to open wav file: {}", e)))?
    .into_samples::<i16>()
    .map(|x| x.map_err(|e| Error::new(&format!("Failed to read wav file: {}", e))))
    .collect::<Result<Vec<i16>, Error>>()?;
```

The Whisper model expects 16KHz mono f32 samples, meaning we have to do just a bit more work:

```rust
let mut inter_samples = vec![Default::default(); samples.len()];
whisper_rs::convert_integer_to_float_audio(&samples, &mut inter_samples)
    .map_err(|e| Error::new(&format!("Failed to convert audio data: {}", e)))?;
let samples = whisper_rs::convert_stereo_to_mono_audio(&inter_samples)
    .map_err(|e| Error::new(&format!("Failed to convert audio data: {}", e)))?;
```

After that's done, we can run the model and get back a vector of text segments. Et voila!

### Summarization

The summarization component interfaces with Ollama to run large language models locally. The model parameters are tuned for consistent, focused output - low temperature reduces randomness while constrained sampling ensures the model stays on topic.

The prompt engineering focuses on extracting actionable information:

```rust
fn get_prompt(&self, text: &str) -> String {
    format!(
        r#"
You are an AI assistant that summarizes meeting transcriptions. Your task is to:

1. Extract the key topics and decisions discussed
2. Identify action items and their owners (if mentioned)
3. Note any important technical details or specifications
4. Highlight any unresolved questions or issues
5. Keep the summary concise but comprehensive

Please provide a well-structured summary of the following meeting transcript:
        {text}
        "#
    )
}
```

## Unix Philosophy and CLI Design

I designed Essence to follow Unix principles: do one thing well and play nicely with other tools. The CLI exposes two primary commands that can be chained together or used independently:

```bash
essence transcribe -i audio.wav -l en -m ggml-large-v3.bin
essence summarize -i transcript.txt -m gemma3:27b
```

Following Unix conventions, the tool outputs results to stdout and logs to stderr. This makes it perfect for scripting and automation. Here's a script that takes a locally recorded meeting video and produces a summary:

```bash
#!/bin/bash
# extract_and_summarize.sh - Convert video to summary

VIDEO_FILE="$1"
AUDIO_FILE="meeting_audio.wav"
TRANSCRIPT_FILE="transcript.txt"

# Extract audio from video
ffmpeg -i "$VIDEO_FILE" -ar 16000 -ac 1 "$AUDIO_FILE" -y

# Transcribe the audio
essence transcribe -i "$AUDIO_FILE" -l en -m ~/models/ggml-large-v3.bin > "$TRANSCRIPT_FILE"

# Generate summary
essence summarize -i "$TRANSCRIPT_FILE" -m gemma3:27b

# Cleanup
rm "$AUDIO_FILE" "$TRANSCRIPT_FILE"
```

Now imagine running this script after each meeting, saving the result in your Obsidian vault, and then chatting with it using the [fully-local RAG pipeline I previously built](https://orellazri.com/posts/rag-pipeline-chat-with-my-obsidian-vault/).
