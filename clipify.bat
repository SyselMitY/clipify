@echo off
setlocal enabledelayedexpansion

:loop

REM Check if a file was dragged onto the script
if "%~1"=="" (
exit /b
)

set "input_file=%~1"
set "audio_file=%~dp1%~n1_audiomerge.mkv"

ffmpeg -y -i "%input_file%" -filter_complex "[a:0][a:2][a:1]amerge=inputs=3[a]" -map "[a]" -map 0:V -c:v copy -ac 2 "%audio_file%"

REM Set input and output variables
set "output_file=%~dp1%~n1_reencode.mp4"

REM Run FFmpeg command

ffmpeg -y -vsync 0 -hwaccel cuda -hwaccel_output_format cuda -i "%audio_file%" -filter:v fps=60,scale_cuda=1280:720 -map 0 -c:a copy -c:v hevc_nvenc -cq 30 -preset p6 -tune hq -g 250 -bf 3 -b_ref_mode middle -temporal-aq 1 -rc-lookahead 40 -i_qfactor 0.75 -b_qfactor 1.1 "%output_file%"

echo Complete!

shift
pause
goto loop
