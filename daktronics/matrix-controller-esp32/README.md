# matrix-controller-esp32

wip rust firmware to drive the led matrix. has a custom driver that implements the `embedded_graphics` `DrawTarget` trait, so it should be easy to get working with other stuff! it also has a captive portal to configure wifi credentials

## hardware

xiao esp32c6

## `bad_apple.rgb`

```shell
ffmpeg -i ./FtutLA63Cp8.webm -vf scale=96:-1,crop=96:16:0:25,hue=s=0,format=gray -r 30 -t 70 -pix_fmt gray8 bad_apple.rgb
```