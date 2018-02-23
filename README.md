# Remarkable Framebuffer Analysis Project

This repository contains a collection of scripts, code and general information on what makes Remarkable Paper Tablet tick, focusing on gaining access to the low latency refresh capabilities of the device which are normally not exposed.

[![PoC](https://thumbs.gfycat.com/GlitteringShortIchneumonfly-size_restricted.gif)](https://gfycat.com/gifs/detail/GlitteringShortIchneumonfly)

(GIF Preview has limited FPS -- click to watch at full framerate)

## Build Instructions
First run `make freetype` to generate the `libfreetype` static build with the expected flags.

Execute `make all` to generate the `poc` executable along with `spy.so`, `libremarkable.so` and `libremarkable.a`.

The makefiles assume the following are available in your `$PATH`, you may need to override or change them if they are installed elsewhere on your system:
```
CC = arm-linux-gnueabihf-gcc
CXX = arm-linux-gnueabihf-g++
AR = arm-linux-gnueabihf-ar
```
The toolchain that would be acquired from either of these sources would be able to cross-compile for the Remarkable Tablet:
```
AUR:         https://aur.archlinux.org/packages/arm-linux-gnueabihf-gcc/
Remarkable:  https://remarkable.engineering/deploy/sdk/poky-glibc-x86_64-meta-toolchain-qt5-cortexa9hf-neon-toolchain-2.1.3.sh
```

## Partial Redraw Proof of Concept (poc)
Contains the proof of concept for directly interacting with the eInk display driver to perform partial updates.

The key finding here is the magic values and their usage in conjunction with the dumped `mxcfb_*` data structures. Simply update the framebuffer and then call `ioctl` on the `/dev/fb0` FD with `REMARKABLE_PREFIX | MXCFB_SEND_UPDATE` in order to quickly the redraw region defined by `data.update_region` and that region only.

```c
#define REMARKABLE_PREFIX                       0x40484600
#define MXCFB_SEND_UPDATE                       0x0000002e
#define MXCFB_WAIT_FOR_VSYNC                    0x00000020
#define MXCFB_SET_GBL_ALPHA                     0x00000021
#define MXCFB_SET_CLR_KEY                       0x00000022
#define MXCFB_SET_OVERLAY_POS                   0x00000024
#define MXCFB_GET_FB_IPU_CHAN                   0x00000025
#define MXCFB_SET_LOC_ALPHA                     0x00000026
#define MXCFB_SET_LOC_ALP_BUF                   0x00000027
#define MXCFB_SET_GAMMA                         0x00000028
#define MXCFB_GET_FB_IPU_DI                     0x00000029
#define MXCFB_GET_DIFMT                         0x0000002a
#define MXCFB_GET_FB_BLANK                      0x0000002b
#define MXCFB_SET_WAVEFORM_MODES                0x0000002b
#define MXCFB_SET_DIFMT                         0x0000002c
#define MXCFB_SET_TEMPERATURE                   0x0000002c
#define MXCFB_SET_AUTO_UPDATE_MODE              0x0000002d
#define MXCFB_WAIT_FOR_UPDATE_COMPLETE	        0x0000002f
#define MXCFB_SET_PWRDOWN_DELAY	                0x00000030
#define MXCFB_GET_PWRDOWN_DELAY	                0x00000031
#define MXCFB_SET_UPDATE_SCHEME                 0x00000032
#define MXCFB_SET_MERGE_ON_WAVEFORM_MISMATCH    0x00000037

mxcfb_update_data data;
data.update_region.top = 0;
data.update_region.left = 0;
data.update_region.width = vinfo.xres;
data.update_region.height = vinfo.yres;
data.waveform_mode = 0x0002;
data.temp = 0x1001;
data.update_mode = 0x0000;
data.update_marker = 0x002a;
data.flags = 0;
data.alt_buffer_data = NULL;
ioctl(fb, REMARKABLE_PREFIX | MXCFB_SEND_UPDATE, &data);
```

## Initial Findings (not up to date, check the code, comments and the latest commits for the latest findings)
Current framebuffer can be dumped with:
```bash
ssh root@10.11.99.1 "cat /dev/fb0" | convert -depth 16 -size 1408x1872+0 gray:- png:/tmp/frame;
```

Remarkable Paper Tablet has an undocumented API for partial refreshes on its eInk display, which is what's behind its magic that disappears when custom Qt applications are used to draw on the screen, even using the toolchain provided by Remarkable.

The `xochitl` program opens `/dev/fb0`, which always ends up being the `FD=3`. It then writes to this FD when it wants to update the screen and uses primarily the following `ioctl` call in order to perform its partial updates when the user draws on the device (`0x4048462e` is the `PARTIAL_UPDATE_MAGIC`, and the next argument is a pointer to `mxcfb_update_data`):

What is particularly interesting here is that `0x4048462e` also happens to be Kindle's `MXCFB_SEND_UPDATE` magic. Something to keep in mind.

```c
typedef struct {
  uint32_t top;    // 0x0000
  uint32_t left;   // 0x0000 
  uint32_t width;  // 0x0258 = 600
  uint32_t height; // 0x0320 = 800
} mxcfb_rect;

typedef struct {
  mxcfb_rect update_region;

  uint32_t waveform_mode;  // 0x0002 = WAVEFORM_MODE_GC16 
  uint32_t update_mode;    // 0x0000 = UPDATE_MODE_PARTIAL 
  uint32_t update_marker;  // 0x002a 
  
  int temp;   // 0x1001 = TEMP_USE_PAPYRUS
  uint flags; // 0x0000 
  
  void* alt_buffer_data; // must not used when flags is 0
  ...
} mxcfb_update_data;

ioctl(3, 0x4048462e, 0x7ea2d290{
   updateRegion: x: 0
                 y: 0
                 width: 1404
                 height: 1872
   waveformMode: 3,
   updateMode:   0
   updateMarker: 46
   temp: 4096
   flags: 0000
   alt_buffer_data: 0x300f30
   ...
}) == 0
```

The `xochitl` program is statically linked with the `QsgEpaperPlugin` which can be found in this repository with the filename `libqsgepaper.a`, containing the following object files, providing the following implementations. These implementations, however, are not used in the PoC as they are not yet fully explored. What is used instead skipping what `libqsgepaper` can achieve with its undocumented portions of the API listed below, and explored all throughout this repository.

However, looking at the function signatures and the analysis so far, it looks like the PoC actually has gotten them right (`EPFrameBuffer::WaveformMode, EPFrameBuffer::UpdateMode` in `EPFramebuffer::sendUpdate`, returning a `uint32_t refresh_marker` that is referred to as an `updateCounter` in `epframebuffer.o`):

```
epcontext.o:
  EPRenderContext::initialize(QOpenGLContext*)
  EPRenderContext::initialize(QSGMaterialShader*)
  EPRenderContext::invalidate()
  EPRenderContext::createRenderer()
  EPRenderContext::renderNextFrame(QSGRenderer*, unsigned int)
  EPRenderContext::EPRenderContext(EPContext*)
  EPContext::createGlyphNode(QSGRenderContext*, bool)
  EPContext::createImageNode()
  EPContext::createPainterNode(QQuickPaintedItem*)
  EPContext::createRectangleNode()
  EPContext::createRenderContext()
  EPContext::createAnimationDriver(QObject*)
  EPContext::renderContextInitialized(QSGRenderContext*)
  EPContext::EPContext(QObject*)
  EPRenderContext::isValid() const
  EPContext::minimumFBOSize() const
  EPContext::defaultSurfaceFormat() const

epframebuffer.o:
  EPFrameBuffer::sendUpdate(QRect, EPFrameBuffer::WaveformMode, EPFrameBuffer::UpdateMode, bool)
  EPFrameBuffer::clearScreen()
  EPFrameBuffer::waitForLastUpdate()
  EPFrameBuffer::instance()
  EPFrameBuffer::EPFrameBuffer()
  EPFrameBuffer::~EPFrameBuffer()
  EPFrameBuffer::sendUpdate(QRect, EPFrameBuffer::WaveformMode, EPFrameBuffer::UpdateMode, bool)::updateCounter
  EPFrameBuffer::instance()::framebuffer

eprenderer.o:
  RendererDebug()
  EPRenderer::nodeChanged(QSGNode*, QFlags<QSGNode::DirtyStateBit>)
  EPRenderer::handleEpaperNode(EPNode*)
  EPRenderer::visit(QSGClipNode*)
  EPRenderer::visit(QSGRootNode*)
  EPRenderer::visit(QSGGlyphNode*)
  EPRenderer::visit(QSGImageNode*)
  EPRenderer::visit(QSGOpacityNode*)
  EPRenderer::visit(QSGPainterNode*)
  EPRenderer::visit(QSGGeometryNode*)
  EPRenderer::visit(QSGNinePatchNode*)
  EPRenderer::visit(QSGRectangleNode*)
  EPRenderer::visit(QSGTransformNode*)
  EPRenderer::endVisit(QSGClipNode*)
  EPRenderer::endVisit(QSGRootNode*)
  EPRenderer::endVisit(QSGGlyphNode*)
  EPRenderer::endVisit(QSGImageNode*)
  EPRenderer::endVisit(QSGOpacityNode*)
  EPRenderer::endVisit(QSGPainterNode*)
  EPRenderer::endVisit(QSGGeometryNode*)
  EPRenderer::endVisit(QSGNinePatchNode*)
  EPRenderer::endVisit(QSGRectangleNode*)
  EPRenderer::endVisit(QSGTransformNode*)
  EPRenderer::EPRenderer(EPRenderContext*)
  EPRenderer::render()
  EPRenderer::drawRects()

eprenderloop.o:
  EPRenderLoop::maybeUpdate(QQuickWindow*)
  EPRenderLoop::exposureChanged(QQuickWindow*)
  EPRenderLoop::windowDestroyed(QQuickWindow*)
  EPRenderLoop::releaseResources(QQuickWindow*)
  EPRenderLoop::handleUpdateRequest(QQuickWindow*)
  EPRenderLoop::grab(QQuickWindow*)
  EPRenderLoop::hide(QQuickWindow*)
  EPRenderLoop::show(QQuickWindow*)
  EPRenderLoop::update(QQuickWindow*)
  EPRenderLoop::EPRenderLoop()
  EPRenderLoop::EPRenderLoop()
  EPRenderLoop::animationDriver() const
  EPRenderLoop::sceneGraphContext() const
  EPRenderLoop::createRenderContext(QSGContext*) const

eptexture.o:
  EPTextureFactory::EPTextureFactory(QImage const&)
  EPTexture::bind()
  EPTexture::EPTexture(QImage const&)
  EPTexture::~EPTexture()
  EPTextureFactory::textureSize() const
  EPTextureFactory::createTexture(QQuickWindow*) const
  EPTextureFactory::textureByteCount() const
  EPTextureFactory::image() const
  EPTexture::hasMipmaps() const
  EPTexture::textureSize() const
  EPTexture::isAtlasTexture() const
  EPTexture::hasAlphaChannel() const
  EPTexture::removedFromAtlas() const
  EPTexture::normalizedTextureSubRect() const
  EPTexture::textureId() const
  EPTexture::EPTexture(QImage const&)::id

epglyphnode.o:
  EPGlyphNode::setStyleColor(QColor const&)
  EPGlyphNode::EPGlyphNodeContent::~EPGlyphNodeContent()
  EPGlyphNode::setPreferredAntialiasingMode(QSGGlyphNode::AntialiasingMode)
  EPGlyphNode::update()
  EPGlyphNode::setColor(QColor const&)
  EPGlyphNode::setStyle(QQuickText::TextStyle)
  EPGlyphNode::setGlyphs(QPointF const&, QGlyphRun const&)
  EPGlyphNode::EPGlyphNode()
  EPGlyphNode::~EPGlyphNode()
  QSGGlyphNode::setBoundingRect(QRectF const&)
  QSGGlyphNode::accept(QSGNodeVisitorEx*)
  QSGNode::preprocess()
  EPGlyphNode::EPGlyphNodeContent::draw(QPainter*) const
  EPGlyphNode::baseLine() const
  QSGGlyphNode::boundingRect() const

epimagenode.o:
  EPImageNode::setTexture(QSGTexture*)
  EPImageNode::setFiltering(QSGTexture::Filtering)
  EPImageNode::setTargetRect(QRectF const&)
  EPImageNode::setSubSourceRect(QRectF const&)
  EPImageNode::EPImageNodeContent::updateCached()
  EPImageNode::EPImageNodeContent::~EPImageNodeContent()
  EPImageNode::setInnerSourceRect(QRectF const&)
  EPImageNode::setInnerTargetRect(QRectF const&)
  EPImageNode::setMipmapFiltering(QSGTexture::Filtering)
  EPImageNode::setVerticalWrapMode(QSGTexture::WrapMode)
  EPImageNode::setHorizontalWrapMode(QSGTexture::WrapMode)
  EPImageNode::update()
  EPImageNode::setMirror(bool)
  EPImageNode::EPImageNode()
  EPImageNode::~EPImageNode()
  QSGImageNode::setAntialiasing(bool)
  QSGImageNode::accept(QSGNodeVisitorEx*)
  QSGNode::preprocess()
  EPImageNode::EPImageNodeContent::draw(QPainter*) const

epnode.o:
  EPNode::Content::~Content()
  EPNode::~EPNode()

eppainternode.o:
  EPPainterNode::setFillColor(QColor const&)
  EPPainterNode::setMipmapping(bool)
  EPPainterNode::setTextureSize(QSize const&)
  EPPainterNode::setContentsScale(double)
  EPPainterNode::setOpaquePainting(bool)
  EPPainterNode::setSmoothPainting(bool)
  EPPainterNode::setFastFBOResizing(bool)
  EPPainterNode::setLinearFiltering(bool)
  EPPainterNode::EPPainterNodeContent::~EPPainterNodeContent()
  EPPainterNode::EPPainterNodeContent::~EPPainterNodeContent()
  EPPainterNode::EPPainterNodeContent::~EPPainterNodeContent()
  EPPainterNode::EPPainterNodeContent::~EPPainterNodeContent()
  EPPainterNode::setPreferredRenderTarget(QQuickPaintedItem::RenderTarget)
  EPPainterNode::update()
  EPPainterNode::setSize(QSize const&)
  EPPainterNode::setDirty(QRect const&)
  EPPainterNode::EPPainterNode(QQuickPaintedItem*)
  EPPainterNode::EPPainterNode(QQuickPaintedItem*)
  EPPainterNode::~EPPainterNode()
  EPPainterNode::~EPPainterNode()
  EPPainterNode::~EPPainterNode()
  EPPainterNode::~EPPainterNode()
  QSGPainterNode::accept(QSGNodeVisitorEx*)
  QSGNode::preprocess()
  EPPainterNode::EPPainterNodeContent::draw(QPainter*) const
  EPPainterNode::texture() const
  EPPainterNode::toImage() const

eprectanglenode.o:
  EPRectangleNode::setAligned(bool)
  EPRectangleNode::setPenColor(QColor const&)
  EPRectangleNode::setPenWidth(double)
  EPRectangleNode::setAntialiasing(bool)
  EPRectangleNode::setGradientStops(QVector<QPair<double, QColor> > const&)
  EPRectangleNode::updateIsGrayscale()
  EPRectangleNode::EPRectangleNodeContent::~EPRectangleNodeContent()
  EPRectangleNode::EPRectangleNodeContent::~EPRectangleNodeContent()
  EPRectangleNode::EPRectangleNodeContent::~EPRectangleNodeContent()
  EPRectangleNode::EPRectangleNodeContent::~EPRectangleNodeContent()
  EPRectangleNode::update()
  EPRectangleNode::setRect(QRectF const&)
  EPRectangleNode::setColor(QColor const&)
  EPRectangleNode::setRadius(double)
  EPRectangleNode::EPRectangleNode()
  EPRectangleNode::EPRectangleNode()
  EPRectangleNode::~EPRectangleNode()
  EPRectangleNode::~EPRectangleNode()
  EPRectangleNode::~EPRectangleNode()
  EPRectangleNode::~EPRectangleNode()
  QSGRectangleNode::setAntialiasing(bool)
  QSGRectangleNode::accept(QSGNodeVisitorEx*)
  QSGNode::preprocess()
  EPRectangleNode::EPRectangleNodeContent::draw(QPainter*) const

qsgepaperplugin.o:
  QsgEpaperPlugin::createWindowManager()
  QsgEpaperPlugin::createTextureFactoryFromImage(QImage const&)
  QsgEpaperPlugin::QsgEpaperPlugin()
  QsgEpaperPlugin::keys() const
  QsgEpaperPlugin::create(QString const&) const

moc_epcontext.o:
  EPRenderContext::staticMetaObject
  EPRenderContext::~EPRenderContext()
  EPContext::staticMetaObject
  EPContext::~EPContext()

moc_epframebuffer.o:
  EPRenderContext::staticMetaObject
  EPRenderContext::~EPRenderContext()
  EPContext::staticMetaObject
  EPContext::~EPContext()

moc_eprenderer.o:
  EPRenderer::staticMetaObject
  EPRenderer::renderComplete()
  EPRenderer::~EPRenderer()
  QSGRenderer::setCustomRenderMode(QByteArray const&)

moc_eprenderloop.o:
  EPRenderLoop::staticMetaObject
  EPRenderLoop::~EPRenderLoop()
  QSGRenderLoop::resize(QQuickWindow*)
  QSGRenderLoop::interleaveIncubation() const

moc_eptexture.o:
  EPTextureFactory::staticMetaObject
  EPTextureFactory::~EPTextureFactory()

moc_qsgepaperplugin.o:
  QsgEpaperPlugin::staticMetaObject
  QsgEpaperPlugin::~QsgEpaperPlugin()
```

## FrameBuffer Spy
A shared library that intercepts and displays undocumented framebuffer refresh ioctl calls for the Remarkable Paper Tablet.
Usage:
```sh
$ systemctl stop xochitl
$ LD_PRELOAD=./spy.so xochitl
...
12:06.842 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 191 ms (~DebugHelperClass() ../git/src/debug.h:16)
ioctl(3, 0x4048462e, 0x7ea2d290{
   updateRegion: x: 0
                 y: 0
                 width: 1404
                 height: 1872
   waveformMode: 3,
   updateMode:   0
   updateMarker: 45
   temp: 4096
   flags: 0000
   alt_buffer_data: 0x300f30
   ...
}) == 0
12:07.207 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 364 ms (~DebugHelperClass() ../git/src/debug.h:16)
12:07.384 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 175 ms (~DebugHelperClass() ../git/src/debug.h:16)
12:07.548 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 162 ms (~DebugHelperClass() ../git/src/debug.h:16)
12:07.705 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 155 ms (~DebugHelperClass() ../git/src/debug.h:16)
ioctl(3, 0x4048462e, 0x7ea2d290{
   updateRegion: x: 0
                 y: 0
                 width: 1404
                 height: 1872
   waveformMode: 3,
   updateMode:   0
   updateMarker: 46
   temp: 4096
   flags: 0000
   alt_buffer_data: 0x300f30
   ...
}) == 0
```

## Findings about the Digitizer / Touch Screen
The device features an ARM SoC from the i.MX6 family by Freescale (--> NXP --> Qualcomm).
```
remarkable: ~/ cat /proc/device-tree/model
reMarkable Prototype 1

remarkable: ~/ cat /proc/device-tree/compatible 
remarkable,zero-gravitasfsl,imx6sl

remarkable: ~/ ls -latr /dev/input/
lrwxrwxrwx    1 root     root             6 Feb 23 05:52 touchscreen0 -> event0
crw-rw----    1 root     input      13,  32 Feb 23 05:52 mouse0
crw-rw----    1 root     input      13,  66 Feb 23 05:52 event2
crw-rw----    1 root     input      13,  65 Feb 23 05:52 event1
crw-rw----    1 root     input      13,  64 Feb 23 05:52 event0
drwxr-xr-x    2 root     root           120 Feb 23 05:52 by-path
drwxr-xr-x    3 root     root           180 Feb 23 05:52 .
crw-rw----    1 root     input      13,  63 Feb 23 05:52 mice
drwxr-xr-x    8 root     root          3460 Feb 23 09:30 ..
```
Events from the touchscreen/digitizer can be seen by reading from these devices.
```
remarkable: ~/ cat /dev/input/mouse0 | hexdump -C
00000000  38 81 81 38 f0 db 28 00  ff 28 00 ff 28 00 ff 38  |8..8..(..(..(..8|
00000010  ff ff 18 ff 00 38 fe ff  18 fe 00 18 fe 00 18 fe  |.....8..........|
00000020  00 38 fe ff 18 fd 00 18  fe 00 18 fd 00 18 fe 00  |.8..............|
00000030  18 fd 00 38 fe ff 18 fd  00 18 fd 00 18 fe 00 18  |...8............|
00000040  fd 00 18 fd 00 18 fe 00  18 fd 00 18 fe 00 18 fd  |................|
00000050  00 18 fb 01 18 fc 01 18  fd 01 18 fd 01 18 fe 01  |................|
00000060  18 fd 01 18 fe 00 18 fe  01 18 fe 01 18 ff 01 18  |................|
00000070  fe 00 18 ff 01 18 ff 00  18 ff 01 18 ff 00 18 ff  |................|
00000080  01 18 ff 00 08 00 01 18  ff 00 08 00 01 08 00 01  |................|

remarkable: ~/ cat /dev/input/event0 | hexdump -C
000025e0  9d 7d 90 5a 2c dc 01 00  03 00 01 00 af 2c 00 00  |.}.Z,........,..|
000025f0  9d 7d 90 5a 2c dc 01 00  03 00 19 00 1d 00 00 00  |.}.Z,...........|
00002600  9d 7d 90 5a 2c dc 01 00  00 00 00 00 00 00 00 00  |.}.Z,...........|
00002610  9d 7d 90 5a 7e f4 01 00  03 00 00 00 e4 14 00 00  |.}.Z~...........|
00002620  9d 7d 90 5a 7e f4 01 00  03 00 01 00 ad 2c 00 00  |.}.Z~........,..|
00002630  9d 7d 90 5a 7e f4 01 00  03 00 19 00 24 00 00 00  |.}.Z~.......$...|
00002640  9d 7d 90 5a 7e f4 01 00  00 00 00 00 00 00 00 00  |.}.Z~...........|
00002650  9d 7d 90 5a c6 02 02 00  03 00 00 00 05 15 00 00  |.}.Z............|
00002660  9d 7d 90 5a c6 02 02 00  03 00 01 00 ac 2c 00 00  |.}.Z.........,..|
00002670  9d 7d 90 5a c6 02 02 00  03 00 19 00 2d 00 00 00  |.}.Z........-...|
00002680  9d 7d 90 5a c6 02 02 00  00 00 00 00 00 00 00 00  |.}.Z............|
00002690  9d 7d 90 5a 02 1c 02 00  03 00 00 00 2d 15 00 00  |.}.Z........-...|
000026a0  9d 7d 90 5a 02 1c 02 00  03 00 01 00 aa 2c 00 00  |.}.Z.........,..|
000026b0  9d 7d 90 5a 02 1c 02 00  03 00 19 00 37 00 00 00  |.}.Z........7...|
000026c0  9d 7d 90 5a 02 1c 02 00  00 00 00 00 00 00 00 00  |.}.Z............|
000026d0  9d 7d 90 5a cf 26 02 00  03 00 00 00 52 15 00 00  |.}.Z.&......R...|
000026e0  9d 7d 90 5a cf 26 02 00  03 00 01 00 a9 2c 00 00  |.}.Z.&.......,..|
000026f0  9d 7d 90 5a cf 26 02 00  00 00 00 00 00 00 00 00  |.}.Z.&..........|
00002700  9d 7d 90 5a 62 3f 02 00  03 00 00 00 76 15 00 00  |.}.Zb?......v...|
00002710  9d 7d 90 5a 62 3f 02 00  03 00 01 00 a7 2c 00 00  |.}.Zb?.......,..|
00002720  9d 7d 90 5a 62 3f 02 00  00 00 00 00 00 00 00 00  |.}.Zb?..........|
00002730  9d 7d 90 5a 4e 66 02 00  01 00 40 01 00 00 00 00  |.}.ZNf....@.....|
00002740  9d 7d 90 5a 4e 66 02 00  00 00 00 00 00 00 00 00  |.}.ZNf..........|
```