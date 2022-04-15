# rshrink 🦀

Application for minimizing file sizes using [imagemagick](https://imagemagick.org/) for images and hopefully soon also [ffmpeg](https://ffmpeg.org/) for videos.

## Current behaviour

```bash
rshrink [FILE_REGEX] [IN_DIR] [OUT_DIR] --dimensions WxH
```

Example usage:

```bash
rshrink .*.(jpg|png|gif) . rshrinked -d 1280x720
```
