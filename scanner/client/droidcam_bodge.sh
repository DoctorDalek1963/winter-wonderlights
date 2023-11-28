#!/usr/bin/env bash

# I use DroidCam for my testing webcam on my main machine.
# Unfortunately, DroidCam provides a webcam with the YU12 format,
# which nokhwa doesn't support. To fix this, we can bodge it.

echo "This script assumes DroidCam is not already connected and the app is open on your phone."
echo "Please plug in your phone and open the DroidCam app, set to use port 47470."
read -p "Press enter to continue..."

set -euxo pipefail

droidcam-cli adb 47470 &> /dev/null & disown

sudo modprobe v4l2loopback
ffmpeg -f v4l2 -input_format yuv420p -i /dev/video0 -c:v mjpeg -f v4l2 /dev/video1 &> /dev/null & disown
