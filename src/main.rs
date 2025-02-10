use std::env;
use std::io::Error;
use std::path::Path;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;

fn main() {
	const MAX_OUTPUT_SIZE: u64 = 10_000_000; // 10 MB
	const CQF_INITIAL: u8 = 20; // initial cqf value
	const CQF_ADJUST_FACTOR: f64 = 5.0; // cqf increase gets scaled by this (in addition to the 3.0 that is expected to halve the file size)

	let args: Vec<String> = env::args().collect();

	if args.len() < 2 {
		eprintln!("No file provided.");
		return;
	}

	let input_path = Path::new(&args[1]);
	let temp_folder = env::temp_dir();

	if !input_path.exists() {
		eprintln!("File does not exist: {}", input_path.display());
		return;
	}

	let audio_path_buf = temp_folder.join(format!(
		"{}_audiomerge.mkv",
		input_path.file_stem().unwrap().to_string_lossy()
	));
	let audio_path = audio_path_buf.as_path();
	let output_path_buf = input_path.with_file_name(format!(
		"{}_reencode.mp4",
		input_path.file_stem().unwrap().to_string_lossy()
	));
	let output_path = output_path_buf.as_path();

	// Merge audio
	println!("Merging audio... ");
	let status = merge_audio(input_path, audio_path).expect("Failed to execute ffmpeg");

	if !status.success() {
		eprintln!("ffmpeg command failed");
		return;
	}

	// Transcode video
	let mut cqf = CQF_INITIAL;
	println!("Transcoding video with CQF {}... ", cqf);
	let status = transcode_video(cqf, audio_path, output_path).expect("Failed to execute ffmpeg");
	if !status.success() {
		eprintln!("ffmpeg command failed");
		return;
	}
	let mut output_file_size = output_path.metadata().unwrap().len();
	println!("Output file size: {} bytes... ", output_file_size);
	
	while output_file_size > MAX_OUTPUT_SIZE {
		// calculate the ratio of the current file size to the target file size
		// adjust the cqf based on the ratio (cqf + 3 equals half the file size)
		let ratio = output_file_size as f64 / MAX_OUTPUT_SIZE as f64;
		let cqf_adjust = (ratio.log2() * 3.0 * CQF_ADJUST_FACTOR).ceil() as u8;
		cqf = cqf + cqf_adjust.max(1);	// ensure that cqf is adjusted by at least 1

		println!("Output too large: ratio = {}, adjusting CQF by {}", ratio, cqf_adjust);

		println!("Transcoding video with CQF {}... ", cqf);

		let status = transcode_video(cqf, audio_path, output_path).expect("Failed to execute ffmpeg");
		if !status.success() {
			eprintln!("ffmpeg command failed");
			return;
		}
		output_file_size = output_path.metadata().unwrap().len();
		println!("Output file size: {} bytes... ", output_file_size);
		std::io::Write::flush(&mut std::io::stdout()).unwrap();
	}


	println!("Complete!");
}

fn merge_audio(input_path: &Path, output_path: &Path) -> Result<ExitStatus, Error> {
	Command::new("ffmpeg")
		.args(&[
			"-y", // Overwrite output file
			"-i", input_path.to_str().unwrap(), // Input file
			"-filter_complex", "[a:0][a:2][a:1]amerge=inputs=3[a]", // Merge audio streams (0: pc audio, 2: mic audio, 1: discord audio)
			"-map", "[a]", // Map the merged audio stream
			"-map", "0:V", // Map the video stream
			"-c:v", "copy", // do not reencode video
			"-ac", "2", // Merge to stereo
			output_path.to_str().unwrap(), // Output file
		])
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
}

fn transcode_video(cqf: u8, input_path: &Path, output_path: &Path) -> Result<ExitStatus, Error> {
	Command::new("ffmpeg")
		.args(&[
			"-y", // Overwrite output file
			"-vsync", "0", // still not sure what this does, something to do with timecodes or something
			"-hwaccel", "cuda", // Use cuda hwdecode
			"-hwaccel_output_format", "cuda", // use vram as output format
			"-i", input_path.to_str().unwrap(), // Input file
			"-filter:v", "fps=60,scale_cuda=1280:720", // downscale and set fps
			"-map", "0", // map all streams from the input
			"-c:a", "copy", // do not reencode audio
			"-c:v", "hevc_nvenc", // use nvenc with hevc for encoding
			"-cq", &cqf.to_string(), // set constant quality factor
			"-preset", "p6", // performance preset
			"-tune", "hq", // set encoder tuning to high quality
			"-g", "250", // max keyframe interval
			"-bf", "3", // set number of b-frames
			"-b_ref_mode", "middle", // set b-frame reference mode
			"-temporal-aq", "1", // temporal adaptive quantization
			"-rc-lookahead", "40", // rate control lookahead
			"-i_qfactor", "0.75", // set i-frame quantization factor
			"-b_qfactor", "1.1", // set b-frame quantization factor
			output_path.to_str().unwrap(),
		])
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
}
