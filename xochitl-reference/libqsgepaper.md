Contents of the `libqsgepaper.a` that `xochitl` is built with:

```c
epcontext.o:
  EPRenderContext::initialize(QOpenGLContext*);
  EPRenderContext::initialize(QSGMaterialShader*);
  EPRenderContext::invalidate();
  EPRenderContext::createRenderer();
  EPRenderContext::renderNextFrame(QSGRenderer*, unsigned int);
  EPRenderContext::EPRenderContext(EPContext*);
  EPContext::createGlyphNode(QSGRenderContext*, bool);
  EPContext::createImageNode();
  EPContext::createPainterNode(QQuickPaintedItem*);
  EPContext::createRectangleNode();
  EPContext::createRenderContext();
  EPContext::createAnimationDriver(QObject*);
  EPContext::renderContextInitialized(QSGRenderContext*);
  EPContext::EPContext(QObject*);
  EPRenderContext::isValid() const;
  EPContext::minimumFBOSize() const;
  EPContext::defaultSurfaceFormat() const;

epframebuffer.o:
  EPFrameBuffer::sendUpdate(QRect, EPFrameBuffer::WaveformMode, EPFrameBuffer::UpdateMode, bool);
  EPFrameBuffer::clearScreen();
  EPFrameBuffer::waitForLastUpdate();
  EPFrameBuffer::instance();
  EPFrameBuffer::EPFrameBuffer();
  EPFrameBuffer::~EPFrameBuffer();
  EPFrameBuffer::sendUpdate(QRect, EPFrameBuffer::WaveformMode, EPFrameBuffer::UpdateMode, bool)::updateCounter
  EPFrameBuffer::instance()::framebuffer

eprenderer.o:
  RendererDebug();
  EPRenderer::nodeChanged(QSGNode*, QFlags<QSGNode::DirtyStateBit>);
  EPRenderer::handleEpaperNode(EPNode*);
  EPRenderer::visit(QSGClipNode*);
  EPRenderer::visit(QSGRootNode*);
  EPRenderer::visit(QSGGlyphNode*);
  EPRenderer::visit(QSGImageNode*);
  EPRenderer::visit(QSGOpacityNode*);
  EPRenderer::visit(QSGPainterNode*);
  EPRenderer::visit(QSGGeometryNode*);
  EPRenderer::visit(QSGNinePatchNode*);
  EPRenderer::visit(QSGRectangleNode*);
  EPRenderer::visit(QSGTransformNode*);
  EPRenderer::endVisit(QSGClipNode*);
  EPRenderer::endVisit(QSGRootNode*);
  EPRenderer::endVisit(QSGGlyphNode*);
  EPRenderer::endVisit(QSGImageNode*);
  EPRenderer::endVisit(QSGOpacityNode*);
  EPRenderer::endVisit(QSGPainterNode*);
  EPRenderer::endVisit(QSGGeometryNode*);
  EPRenderer::endVisit(QSGNinePatchNode*);
  EPRenderer::endVisit(QSGRectangleNode*);
  EPRenderer::endVisit(QSGTransformNode*);
  EPRenderer::EPRenderer(EPRenderContext*);
  EPRenderer::render();
  EPRenderer::drawRects();

eprenderloop.o:
  EPRenderLoop::maybeUpdate(QQuickWindow*);
  EPRenderLoop::exposureChanged(QQuickWindow*);
  EPRenderLoop::windowDestroyed(QQuickWindow*);
  EPRenderLoop::releaseResources(QQuickWindow*);
  EPRenderLoop::handleUpdateRequest(QQuickWindow*);
  EPRenderLoop::grab(QQuickWindow*);
  EPRenderLoop::hide(QQuickWindow*);
  EPRenderLoop::show(QQuickWindow*);
  EPRenderLoop::update(QQuickWindow*);
  EPRenderLoop::EPRenderLoop();
  EPRenderLoop::EPRenderLoop();
  EPRenderLoop::animationDriver() const;
  EPRenderLoop::sceneGraphContext() const;
  EPRenderLoop::createRenderContext(QSGContext*) const;

eptexture.o:
  EPTextureFactory::EPTextureFactory(QImage const;&);
  EPTexture::bind();
  EPTexture::EPTexture(QImage const;&);
  EPTexture::~EPTexture();
  EPTextureFactory::textureSize() const;
  EPTextureFactory::createTexture(QQuickWindow*) const;
  EPTextureFactory::textureByteCount() const;
  EPTextureFactory::image() const;
  EPTexture::hasMipmaps() const;
  EPTexture::textureSize() const;
  EPTexture::isAtlasTexture() const;
  EPTexture::hasAlphaChannel() const;
  EPTexture::removedFromAtlas() const;
  EPTexture::normalizedTextureSubRect() const;
  EPTexture::textureId() const;
  EPTexture::EPTexture(QImage const;&)::id

epglyphnode.o:
  EPGlyphNode::setStyleColor(QColor const;&);
  EPGlyphNode::EPGlyphNodeContent::~EPGlyphNodeContent();
  EPGlyphNode::setPreferredAntialiasingMode(QSGGlyphNode::AntialiasingMode);
  EPGlyphNode::update();
  EPGlyphNode::setColor(QColor const;&);
  EPGlyphNode::setStyle(QQuickText::TextStyle);
  EPGlyphNode::setGlyphs(QPointF const;&, QGlyphRun const;&);
  EPGlyphNode::EPGlyphNode();
  EPGlyphNode::~EPGlyphNode();
  QSGGlyphNode::setBoundingRect(QRectF const;&);
  QSGGlyphNode::accept(QSGNodeVisitorEx*);
  QSGNode::preprocess();
  EPGlyphNode::EPGlyphNodeContent::draw(QPainter*) const;
  EPGlyphNode::baseLine() const;
  QSGGlyphNode::boundingRect() const;

epimagenode.o:
  EPImageNode::setTexture(QSGTexture*);
  EPImageNode::setFiltering(QSGTexture::Filtering);
  EPImageNode::setTargetRect(QRectF const;&);
  EPImageNode::setSubSourceRect(QRectF const;&);
  EPImageNode::EPImageNodeContent::updateCached();
  EPImageNode::EPImageNodeContent::~EPImageNodeContent();
  EPImageNode::setInnerSourceRect(QRectF const;&);
  EPImageNode::setInnerTargetRect(QRectF const;&);
  EPImageNode::setMipmapFiltering(QSGTexture::Filtering);
  EPImageNode::setVerticalWrapMode(QSGTexture::WrapMode);
  EPImageNode::setHorizontalWrapMode(QSGTexture::WrapMode);
  EPImageNode::update();
  EPImageNode::setMirror(bool);
  EPImageNode::EPImageNode();
  EPImageNode::~EPImageNode();
  QSGImageNode::setAntialiasing(bool);
  QSGImageNode::accept(QSGNodeVisitorEx*);
  QSGNode::preprocess();
  EPImageNode::EPImageNodeContent::draw(QPainter*) const;

epnode.o:
  EPNode::Content::~Content();
  EPNode::~EPNode();

eppainternode.o:
  EPPainterNode::setFillColor(QColor const;&);
  EPPainterNode::setMipmapping(bool);
  EPPainterNode::setTextureSize(QSize const;&);
  EPPainterNode::setContentsScale(double);
  EPPainterNode::setOpaquePainting(bool);
  EPPainterNode::setSmoothPainting(bool);
  EPPainterNode::setFastFBOResizing(bool);
  EPPainterNode::setLinearFiltering(bool);
  EPPainterNode::EPPainterNodeContent::~EPPainterNodeContent();
  EPPainterNode::setPreferredRenderTarget(QQuickPaintedItem::RenderTarget);
  EPPainterNode::update();
  EPPainterNode::setSize(QSize const;&);
  EPPainterNode::setDirty(QRect const;&);
  EPPainterNode::EPPainterNode(QQuickPaintedItem*);
  EPPainterNode::~EPPainterNode();
  QSGPainterNode::accept(QSGNodeVisitorEx*);
  QSGNode::preprocess();
  EPPainterNode::EPPainterNodeContent::draw(QPainter*) const;
  EPPainterNode::texture() const;
  EPPainterNode::toImage() const;

eprectanglenode.o:
  EPRectangleNode::setAligned(bool);
  EPRectangleNode::setPenColor(QColor const;&);
  EPRectangleNode::setPenWidth(double);
  EPRectangleNode::setAntialiasing(bool);
  EPRectangleNode::setGradientStops(QVector<QPair<double, QColor> > const;&);
  EPRectangleNode::updateIsGrayscale();
  EPRectangleNode::EPRectangleNodeContent::~EPRectangleNodeContent();
  EPRectangleNode::update();
  EPRectangleNode::setRect(QRectF const;&);
  EPRectangleNode::setColor(QColor const;&);
  EPRectangleNode::setRadius(double);
  EPRectangleNode::EPRectangleNode();
  EPRectangleNode::~EPRectangleNode();
  QSGRectangleNode::setAntialiasing(bool);
  QSGRectangleNode::accept(QSGNodeVisitorEx*);
  QSGNode::preprocess();
  EPRectangleNode::EPRectangleNodeContent::draw(QPainter*) const;

qsgepaperplugin.o:
  QsgEpaperPlugin::createWindowManager();
  QsgEpaperPlugin::createTextureFactoryFromImage(QImage const;&);
  QsgEpaperPlugin::QsgEpaperPlugin();
  QsgEpaperPlugin::keys() const;
  QsgEpaperPlugin::create(QString const;&) const;

moc_epcontext.o:
  EPRenderContext::staticMetaObject;
  EPRenderContext::~EPRenderContext();
  EPContext::staticMetaObject;
  EPContext::~EPContext();

moc_epframebuffer.o:
  EPRenderContext::staticMetaObject;
  EPRenderContext::~EPRenderContext();
  EPContext::staticMetaObject;
  EPContext::~EPContext();

moc_eprenderer.o:
  EPRenderer::staticMetaObject;
  EPRenderer::renderComplete();
  EPRenderer::~EPRenderer();
  QSGRenderer::setCustomRenderMode(QByteArray const;&);

moc_eprenderloop.o:
  EPRenderLoop::staticMetaObject;
  EPRenderLoop::~EPRenderLoop();
  QSGRenderLoop::resize(QQuickWindow*);
  QSGRenderLoop::interleaveIncubation() const;

moc_eptexture.o:
  EPTextureFactory::staticMetaObject;
  EPTextureFactory::~EPTextureFactory();

moc_qsgepaperplugin.o:
  QsgEpaperPlugin::staticMetaObject;
  QsgEpaperPlugin::~QsgEpaperPlugin();
```
