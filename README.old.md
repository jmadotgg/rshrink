# rshrink 🦀

Application for minimizing file sizes using [imagemagick](https://imagemagick.org/) for images and hopefully soon also [ffmpeg](https://ffmpeg.org/) for videos.

## Current behaviour

```bash
rshrink [FILE_REGEX] [IN_DIR] [OUT_DIR] --dimensions WxH --gaussian_blur [BOOLEAN] --quality [INTEGER]
```

Example usage:

```bash
rshrink .*.(jpg|png) . rshrinked -d 1280x720 -g true -q 50
```

Default positional argument values:

| Argument position | Argument name | Default value   |
| ----------------- | ------------- | --------------- |
| 1                 | FILE_REGEX    | `.*.(jpg\|png)` |
| 2                 | IN_DIR        | `.`             |
| 3                 | OUT_DIR       | `_rshrinked`    |

Default flag values:

| Flag long         | Flag short | Default value | Additional notes                                                 |
| ----------------- | ---------- | ------------- | ---------------------------------------------------------------- |
| `--dimensions`    | `-d`       | [ORIGINAL]    | Scales image to fit dimensions (preserves original aspect ratio) |
| `--gaussian_blur` | `-g`       | `false`       | slow (not recommended)                                           |
| `--quality`       | `-q`       | `85`          | Compression quality                                              |
