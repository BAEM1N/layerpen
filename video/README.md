# Pointory product introduction

**Archived draft.** The rendered MP4 was withdrawn from the public v0.1 release and root README on 2026-09-09. This source is retained for redesign; it is not the current product demonstration. Research and a new storyboard come before rendering a replacement.

36 seconds, 1920 × 1080, 30 fps, H.264 MP4. English on-screen copy, no music or voiceover. Settings, drawing and zoom scenes are product illustrations; they are not screen recordings. No confidential screen captures or external stock assets are used.

## Edit on macOS or Windows

Install Node.js 22 or later, then:

```sh
cd video
npm ci
npm run studio
```

Edit `src/index.jsx`. Each of the six scenes lasts 180 frames: introduction, drawing, frozen-region zoom, languages, exports, and the GitHub download call to action.

```sh
npm run stills
npm run render
```

The output is `out/Pointory-intro-v0.1.0.mp4`. Remotion can download its supported browser automatically. To use an installed Chrome instead, set `REMOTION_BROWSER_EXECUTABLE` to its full executable path. On macOS that is usually `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.

The app itself does not depend on Remotion. Remotion has its own license, separate from this project's MIT license; see https://www.remotion.dev/license. Rendering does not publish the video or use a paid cloud renderer.

References: https://www.remotion.dev/docs/bundler and https://www.remotion.dev/docs/renderer/render-media.
