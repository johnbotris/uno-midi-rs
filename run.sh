#!/bin/sh

case $1 in
    midi) cargo r && amidi --dump -p hw:2,0,0;;
    serial) cargo r && minicom -b 9600 -D /dev/ttyACM0;;
    rustc) cargo rustc -- -A warnings;;
    *) cargo b;;
esac
