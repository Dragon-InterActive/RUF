#include "ruf/cpp/ruf_widgets.h"

// Von cxx-qt generiert: vollständige Definitionen der gemeinsamen Enums und
// die Rust-Funktionen zum Aufruf der Callbacks.
#include "ruf/src/ffi.cxxqt.h"

#include <QtCore/QEvent>
#include <QtCore/QMetaObject>
#include <QtCore/QRegularExpression>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontInfo>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeySequence>
#include <QtGui/QPixmap>
#include <QtGui/QResizeEvent>
#include <QtGui/QShortcut>
#include <QtGui/QStyleHints>
#include <QtGui/QSyntaxHighlighter>
#include <QtGui/QColor>
#include <QtGui/QTextBlock>
#include <QtGui/QTextCharFormat>
#include <QtGui/QTextCursor>
#include <QtGui/QTextDocument>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QTabWidget>

namespace ruf {

namespace {

// Kennzeichnet von RUF erzeugte Widgets, damit widget_kind() sie zuordnen kann.
constexpr char const* kKindProperty = "_ruf_kind";

QWidget* create(WidgetKind kind, QWidget* parent)
{
    QWidget* widget = nullptr;
    switch (kind) {
    case WidgetKind::Window:
        widget = new QWidget(parent, Qt::Window);
        break;
    case WidgetKind::Container:
        widget = new QWidget(parent);
        break;
    case WidgetKind::PushButton:
        widget = new QPushButton(parent);
        break;
    case WidgetKind::Label:
        widget = new QLabel(parent);
        break;
    case WidgetKind::LineEdit:
        widget = new QLineEdit(parent);
        break;
    case WidgetKind::CheckBox:
        widget = new QCheckBox(parent);
        break;
    case WidgetKind::RadioButton:
        widget = new QRadioButton(parent);
        break;
    case WidgetKind::GroupBox:
        widget = new QGroupBox(parent);
        break;
    case WidgetKind::TextArea:
        widget = new QPlainTextEdit(parent);
        break;
    case WidgetKind::ComboBox:
        widget = new QComboBox(parent);
        break;
    case WidgetKind::ListBox:
        widget = new QListWidget(parent);
        break;
    case WidgetKind::SpinBox:
        widget = new QSpinBox(parent);
        break;
    case WidgetKind::Slider:
        widget = new QSlider(Qt::Horizontal, parent);
        break;
    case WidgetKind::ProgressBar: {
        auto* bar = new QProgressBar(parent);
        bar->setValue(0); // Qt startet sonst mit -1 ("zurückgesetzt")
        widget = bar;
        break;
    }
    case WidgetKind::TabView:
        widget = new QTabWidget(parent);
        break;
    case WidgetKind::ScrollArea: {
        // Der Inhalt wächst mit dem sichtbaren Bereich, mindestens aber auf seine
        // Mindestgröße – darüber hinaus wird gescrollt.
        auto* area = new QScrollArea(parent);
        area->setWidgetResizable(true);
        area->setWidget(create(WidgetKind::Container, nullptr));
        widget = area;
        break;
    }
    default:
        widget = new QWidget(parent);
        kind = WidgetKind::Container;
        break;
    }
    widget->setProperty(kKindProperty, static_cast<int>(kind));
    return widget;
}

// Qt kopiert Funktoren beim connect(); rust::Box ist nur verschiebbar.
// Ein shared_ptr macht den Funktor kopierbar, die Box bleibt einmalig.
template <typename T>
std::shared_ptr<rust::Box<T>> share(rust::Box<T> box)
{
    return std::make_shared<rust::Box<T>>(std::move(box));
}

// Meldet Größenänderungen des beobachteten Widgets. Kind des Widgets, lebt
// also genau so lange wie dieses.
class ResizeFilter : public QObject {
public:
    ResizeFilter(QWidget* widget, std::shared_ptr<rust::Box<RufSizeCallback>> callback)
        : QObject(widget)
        , m_callback(std::move(callback))
    {
    }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        if (event->type() == QEvent::Resize && watched == parent()) {
            auto* resize = static_cast<QResizeEvent*>(event);
            invoke_size_callback(**m_callback, resize->size().width(), resize->size().height());
        }
        return QObject::eventFilter(watched, event);
    }

private:
    std::shared_ptr<rust::Box<RufSizeCallback>> m_callback;
};

// Fragt vor dem Schließen nach; ein abgelehntes Close-Event bleibt ignoriert.
class CloseFilter : public QObject {
public:
    CloseFilter(QWidget* widget, std::shared_ptr<rust::Box<RufCloseCallback>> callback)
        : QObject(widget)
        , m_callback(std::move(callback))
    {
    }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        if (event->type() == QEvent::Close && watched == parent()) {
            if (!invoke_close_callback(**m_callback)) {
                event->ignore();
                return true;
            }
        }
        return QObject::eventFilter(watched, event);
    }

private:
    std::shared_ptr<rust::Box<RufCloseCallback>> m_callback;
};

// Einfache, zeilenweise Syntax-Hervorhebung über reguläre Ausdrücke.
// Spätere Regeln überschreiben frühere, falls sie dieselben Zeichen treffen.
class RuleHighlighter : public QSyntaxHighlighter {
public:
    explicit RuleHighlighter(QTextDocument* document)
        : QSyntaxHighlighter(document)
    {
        setObjectName(QStringLiteral("_ruf_highlighter"));
    }

    void addRule(QRegularExpression pattern, QTextCharFormat format)
    {
        m_rules.push_back({ std::move(pattern), std::move(format) });
        rehighlight();
    }

    void clearRules()
    {
        m_rules.clear();
        rehighlight();
    }

protected:
    void highlightBlock(QString const& text) override
    {
        for (auto const& rule : m_rules) {
            auto it = rule.pattern.globalMatch(text);
            while (it.hasNext()) {
                auto const match = it.next();
                if (match.capturedLength() > 0) {
                    setFormat(match.capturedStart(), match.capturedLength(), rule.format);
                }
            }
        }
    }

private:
    struct Rule {
        QRegularExpression pattern;
        QTextCharFormat format;
    };
    std::vector<Rule> m_rules;
};

RuleHighlighter* highlighter(QPlainTextEdit* area, bool create)
{
    auto* existing = area->document()->findChild<QSyntaxHighlighter*>(
        QStringLiteral("_ruf_highlighter"), Qt::FindDirectChildrenOnly);
    if (auto* rules = dynamic_cast<RuleHighlighter*>(existing)) {
        return rules;
    }
    return create ? new RuleHighlighter(area->document()) : nullptr;
}

} // namespace

// ---------------------------------------------------------------------------
// Anwendung
// ---------------------------------------------------------------------------

Application::Application(rust::Vec<rust::String> const& args)
    : m_argc(0)
{
    for (auto const& arg : args) {
        m_args.emplace_back(std::string(arg));
    }
    if (m_args.empty()) {
        m_args.emplace_back("ruf");
    }
    for (auto& arg : m_args) {
        m_argv.push_back(arg.data());
    }
    m_argv.push_back(nullptr);
    m_argc = static_cast<int>(m_args.size());
    m_app = std::make_unique<QApplication>(m_argc, m_argv.data());
}

Application::~Application() = default;

std::unique_ptr<Application> application_new(rust::Vec<rust::String> const& args)
{
    return std::make_unique<Application>(args);
}

bool application_exists()
{
    return qobject_cast<QApplication*>(QCoreApplication::instance()) != nullptr;
}

std::int32_t application_exec()
{
    return QApplication::exec();
}

void application_quit()
{
    QCoreApplication::quit();
}

void application_process_events()
{
    QCoreApplication::processEvents();
    // Außerhalb von exec() verarbeitet processEvents() kein deleteLater();
    // ohne das blieben per delete_later() entfernte Widgets bis zum Ende bestehen.
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
}

void application_set_name(QString const& name)
{
    QCoreApplication::setApplicationName(name);
}

QString application_name()
{
    return QCoreApplication::applicationName();
}

ColorScheme application_color_scheme()
{
#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    switch (QGuiApplication::styleHints()->colorScheme()) {
    case Qt::ColorScheme::Light:
        return ColorScheme::Light;
    case Qt::ColorScheme::Dark:
        return ColorScheme::Dark;
    default:
        return ColorScheme::Unknown;
    }
#else
    return ColorScheme::Unknown;
#endif
}

bool application_set_color_scheme(ColorScheme scheme)
{
#if QT_VERSION >= QT_VERSION_CHECK(6, 8, 0)
    Qt::ColorScheme qt = Qt::ColorScheme::Unknown;
    if (scheme == ColorScheme::Light) {
        qt = Qt::ColorScheme::Light;
    } else if (scheme == ColorScheme::Dark) {
        qt = Qt::ColorScheme::Dark;
    }
    QGuiApplication::styleHints()->setColorScheme(qt);
    return true;
#else
    Q_UNUSED(scheme);
    return false;
#endif
}

bool post_to_gui(rust::Box<RufSendCallback> callback)
{
    QCoreApplication* app = QCoreApplication::instance();
    if (!app) {
        return false;
    }
    auto cb = share(std::move(callback));
    // Queued: der Funktor läuft im Thread der Anwendung, also im GUI-Thread.
    QMetaObject::invokeMethod(app, [cb]() { invoke_send_callback(**cb); }, Qt::QueuedConnection);
    return true;
}

void dialog_message(WidgetHandle const& parent, std::int32_t kind, QString const& title, QString const& text)
{
    switch (kind) {
    case 1:
        QMessageBox::warning(parent.get(), title, text);
        break;
    case 2:
        QMessageBox::critical(parent.get(), title, text);
        break;
    default:
        QMessageBox::information(parent.get(), title, text);
        break;
    }
}

bool dialog_confirm(WidgetHandle const& parent, QString const& title, QString const& text)
{
    return QMessageBox::question(parent.get(), title, text) == QMessageBox::Yes;
}

// ---------------------------------------------------------------------------
// Timer
// ---------------------------------------------------------------------------

TimerHandle::TimerHandle()
    : m_timer(std::make_unique<QTimer>())
{
}

TimerHandle::~TimerHandle()
{
    // Ohne QApplication gibt es auch keinen Timer mehr, der feuern könnte.
    if (QCoreApplication::instance() == nullptr) {
        (void)m_timer.release();
    }
}

std::unique_ptr<TimerHandle> timer_new()
{
    return std::make_unique<TimerHandle>();
}

void timer_connect_timeout(TimerHandle const& timer, rust::Box<RufCallback> callback)
{
    auto cb = share(std::move(callback));
    QObject::connect(timer.get(), &QTimer::timeout, timer.get(), [cb]() { invoke_callback(**cb); });
}

void timer_start(TimerHandle const& timer, std::int32_t msec)
{
    timer.get()->start(msec);
}

void timer_stop(TimerHandle const& timer)
{
    timer.get()->stop();
}

bool timer_is_active(TimerHandle const& timer)
{
    return timer.get()->isActive();
}

void timer_set_single_shot(TimerHandle const& timer, bool singleShot)
{
    timer.get()->setSingleShot(singleShot);
}

bool timer_is_single_shot(TimerHandle const& timer)
{
    return timer.get()->isSingleShot();
}

std::int32_t timer_interval(TimerHandle const& timer)
{
    return timer.get()->interval();
}

void timer_single_shot(std::int32_t msec, rust::Box<RufCallback> callback)
{
    auto cb = share(std::move(callback));
    QTimer::singleShot(msec, QCoreApplication::instance(), [cb]() { invoke_callback(**cb); });
}

std::int32_t qt_compile_version()
{
    return QT_VERSION;
}

QString qt_runtime_version()
{
    return QString::fromLatin1(qVersion());
}

// ---------------------------------------------------------------------------
// Widget-Handle
// ---------------------------------------------------------------------------

WidgetHandle::WidgetHandle(QWidget* widget, bool owned)
    : m_widget(widget)
    , m_owned(owned)
{
}

WidgetHandle::~WidgetHandle()
{
    // Ohne QApplication darf kein Widget mehr gelöscht werden – dann lieber
    // ein Leck beim Programmende als ein Absturz.
    if (m_owned && m_widget && m_widget->parentWidget() == nullptr
        && QCoreApplication::instance() != nullptr) {
        delete m_widget.data();
    }
}

std::unique_ptr<WidgetHandle> widget_create(WidgetKind kind)
{
    return std::make_unique<WidgetHandle>(create(kind, nullptr), true);
}

std::unique_ptr<WidgetHandle> widget_create_child(WidgetKind kind, WidgetHandle const& parent)
{
    QWidget* widget = create(kind, parent.get());
    // Qt zeigt Kinder nur mit dem Elternteil an. Wird ein Kind erst danach erzeugt
    // (dynamische Oberflächen), bliebe es sonst unsichtbar.
    if (parent.get() && parent.get()->isVisible() && !widget->isWindow()) {
        widget->show();
    }
    return std::make_unique<WidgetHandle>(widget, true);
}

std::unique_ptr<WidgetHandle> widget_wrap(QWidget* widget)
{
    // Fremd erzeugte Widgets gehören weiterhin ihrem Ersteller.
    return std::make_unique<WidgetHandle>(widget, false);
}

QWidget* widget_raw(WidgetHandle const& handle)
{
    return handle.get();
}

bool widget_is_alive(WidgetHandle const& handle)
{
    return handle.get() != nullptr;
}

WidgetKind widget_kind(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (!w) {
        return WidgetKind::Container;
    }
    QVariant kind = w->property(kKindProperty);
    if (kind.isValid()) {
        return static_cast<WidgetKind>(kind.toInt());
    }
    if (qobject_cast<QPushButton*>(w)) {
        return WidgetKind::PushButton;
    }
    if (qobject_cast<QCheckBox*>(w)) {
        return WidgetKind::CheckBox;
    }
    if (qobject_cast<QRadioButton*>(w)) {
        return WidgetKind::RadioButton;
    }
    if (qobject_cast<QLabel*>(w)) {
        return WidgetKind::Label;
    }
    if (qobject_cast<QLineEdit*>(w)) {
        return WidgetKind::LineEdit;
    }
    if (qobject_cast<QGroupBox*>(w)) {
        return WidgetKind::GroupBox;
    }
    if (qobject_cast<QPlainTextEdit*>(w)) {
        return WidgetKind::TextArea;
    }
    if (qobject_cast<QComboBox*>(w)) {
        return WidgetKind::ComboBox;
    }
    if (qobject_cast<QListWidget*>(w)) {
        return WidgetKind::ListBox;
    }
    if (qobject_cast<QSpinBox*>(w)) {
        return WidgetKind::SpinBox;
    }
    if (qobject_cast<QSlider*>(w)) {
        return WidgetKind::Slider;
    }
    if (qobject_cast<QProgressBar*>(w)) {
        return WidgetKind::ProgressBar;
    }
    if (qobject_cast<QTabWidget*>(w)) {
        return WidgetKind::TabView;
    }
    if (qobject_cast<QScrollArea*>(w)) {
        return WidgetKind::ScrollArea;
    }
    return w->isWindow() ? WidgetKind::Window : WidgetKind::Container;
}

QString widget_class_name(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    return w ? QString::fromLatin1(w->metaObject()->className()) : QString();
}

void widget_set_parent(WidgetHandle const& handle, WidgetHandle const& parent)
{
    if (QWidget* w = handle.get()) {
        bool const wasVisible = w->isVisible();
        w->setParent(parent.get(), w->windowFlags() & ~Qt::WindowType_Mask);
        if (wasVisible) {
            w->show();
        }
    }
}

void widget_set_geometry(WidgetHandle const& handle, QRect const& rect)
{
    if (QWidget* w = handle.get()) {
        w->setGeometry(rect);
    }
}

QRect widget_geometry(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    return w ? w->geometry() : QRect();
}

void widget_set_minimum_size(WidgetHandle const& handle, std::int32_t width, std::int32_t height)
{
    if (QWidget* w = handle.get()) {
        w->setMinimumSize(width, height);
    }
}

void widget_adjust_size(WidgetHandle const& handle)
{
    if (QWidget* w = handle.get()) {
        w->adjustSize();
    }
}

void widget_set_visible(WidgetHandle const& handle, bool visible)
{
    if (QWidget* w = handle.get()) {
        w->setVisible(visible);
    }
}

bool widget_is_visible(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    return w && w->isVisible();
}

bool widget_is_hidden(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    return !w || w->isHidden();
}

void widget_set_enabled(WidgetHandle const& handle, bool enabled)
{
    if (QWidget* w = handle.get()) {
        w->setEnabled(enabled);
    }
}

bool widget_is_enabled(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    return w && w->isEnabled();
}

void widget_set_tool_tip(WidgetHandle const& handle, QString const& text)
{
    if (QWidget* w = handle.get()) {
        w->setToolTip(text);
    }
}

QString widget_tool_tip(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    return w ? w->toolTip() : QString();
}

void widget_set_style_sheet(WidgetHandle const& handle, QString const& css)
{
    if (QWidget* w = handle.get()) {
        w->setStyleSheet(css);
    }
}

QString widget_style_sheet(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    return w ? w->styleSheet() : QString();
}

void widget_set_object_name(WidgetHandle const& handle, QString const& name)
{
    if (QWidget* w = handle.get()) {
        w->setObjectName(name);
    }
}

QString widget_object_name(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    return w ? w->objectName() : QString();
}

void widget_close(WidgetHandle const& handle)
{
    if (QWidget* w = handle.get()) {
        w->close();
    }
}

bool widget_save_screenshot(WidgetHandle const& handle, QString const& path)
{
    QWidget* w = handle.get();
    if (!w) {
        return false;
    }
    w->ensurePolished();
    return w->grab().save(path);
}

void widget_delete_later(WidgetHandle const& handle)
{
    if (QWidget* w = handle.get()) {
        w->hide();
        w->deleteLater();
    }
}

bool widget_add_shortcut(WidgetHandle const& handle, QString const& sequence, rust::Box<RufCallback> callback)
{
    QWidget* w = handle.get();
    QKeySequence const keys(sequence);
    if (!w || keys.isEmpty()) {
        return false;
    }
    auto* shortcut = new QShortcut(keys, w);
    shortcut->setContext(Qt::WindowShortcut);
    auto cb = share(std::move(callback));
    QObject::connect(shortcut, &QShortcut::activated, shortcut, [cb]() { invoke_callback(**cb); });
    return true;
}

std::unique_ptr<WidgetHandle> widget_scroll_content(WidgetHandle const& handle)
{
    auto* area = qobject_cast<QScrollArea*>(handle.get());
    // Der Inhalt gehört der Scrollfläche, nicht dem Handle.
    return std::make_unique<WidgetHandle>(area ? area->widget() : nullptr, false);
}

void widget_set_monospace(WidgetHandle const& handle, bool monospace)
{
    QWidget* w = handle.get();
    if (!w) {
        return;
    }
    if (monospace) {
        QFont font = QFontDatabase::systemFont(QFontDatabase::FixedFont);
        font.setStyleHint(QFont::TypeWriter);
        font.setPointSizeF(w->font().pointSizeF());
        w->setFont(font);
    } else {
        w->setFont(QFont());
    }
    // Merken, was gewünscht war: die tatsächlich aufgelöste Schrift hängt von
    // der Plattform ab (offscreen gibt es z. B. keine echte Schriftauswahl).
    w->setProperty("_ruf_monospace", monospace);
}

bool widget_is_monospace(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (!w) {
        return false;
    }
    QVariant const wanted = w->property("_ruf_monospace");
    return wanted.isValid() ? wanted.toBool() : QFontInfo(w->font()).fixedPitch();
}

bool widget_add_highlight_rule(WidgetHandle const& handle, QString const& pattern, QString const& color, bool bold)
{
    auto* area = qobject_cast<QPlainTextEdit*>(handle.get());
    QRegularExpression const regex(pattern);
    QColor const qcolor = QColor::fromString(color);
    if (!area || !regex.isValid() || pattern.isEmpty() || !qcolor.isValid()) {
        return false;
    }
    QTextCharFormat format;
    format.setForeground(qcolor);
    if (bold) {
        format.setFontWeight(QFont::Bold);
    }
    highlighter(area, true)->addRule(regex, format);
    return true;
}

void widget_clear_highlight_rules(WidgetHandle const& handle)
{
    if (auto* area = qobject_cast<QPlainTextEdit*>(handle.get())) {
        if (auto* rules = highlighter(area, false)) {
            rules->clearRules();
        }
    }
}

void widget_set_marked_line(WidgetHandle const& handle, std::int32_t line, QString const& color)
{
    auto* area = qobject_cast<QPlainTextEdit*>(handle.get());
    if (!area) {
        return;
    }
    QList<QTextEdit::ExtraSelection> selections;
    QTextBlock const block = area->document()->findBlockByNumber(line);
    QColor const qcolor = QColor::fromString(color);
    if (line >= 0 && block.isValid() && qcolor.isValid()) {
        QTextEdit::ExtraSelection selection;
        selection.format.setBackground(qcolor);
        selection.format.setProperty(QTextFormat::FullWidthSelection, true);
        selection.cursor = QTextCursor(block);
        selections.append(selection);
    }
    area->setExtraSelections(selections);
}

void widget_goto_line(WidgetHandle const& handle, std::int32_t line)
{
    auto* area = qobject_cast<QPlainTextEdit*>(handle.get());
    if (!area) {
        return;
    }
    QTextBlock const block = area->document()->findBlockByNumber(line);
    if (block.isValid()) {
        area->setTextCursor(QTextCursor(block));
        area->ensureCursorVisible();
    }
}

void widget_set_text(WidgetHandle const& handle, QString const& text)
{
    QWidget* w = handle.get();
    if (!w) {
        return;
    }
    if (auto* button = qobject_cast<QAbstractButton*>(w)) {
        button->setText(text);
    } else if (auto* label = qobject_cast<QLabel*>(w)) {
        label->setText(text);
    } else if (auto* edit = qobject_cast<QLineEdit*>(w)) {
        edit->setText(text);
    } else if (auto* group = qobject_cast<QGroupBox*>(w)) {
        group->setTitle(text);
    } else if (auto* area = qobject_cast<QPlainTextEdit*>(w)) {
        area->setPlainText(text);
    } else if (auto* combo = qobject_cast<QComboBox*>(w)) {
        // Editierbar: freier Text; sonst wird der passende Eintrag ausgewählt.
        if (combo->isEditable()) {
            combo->setEditText(text);
        } else {
            combo->setCurrentText(text);
        }
    } else {
        w->setWindowTitle(text);
    }
}

QString widget_text(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (!w) {
        return QString();
    }
    if (auto* button = qobject_cast<QAbstractButton*>(w)) {
        return button->text();
    }
    if (auto* label = qobject_cast<QLabel*>(w)) {
        return label->text();
    }
    if (auto* edit = qobject_cast<QLineEdit*>(w)) {
        return edit->text();
    }
    if (auto* group = qobject_cast<QGroupBox*>(w)) {
        return group->title();
    }
    if (auto* area = qobject_cast<QPlainTextEdit*>(w)) {
        return area->toPlainText();
    }
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        return combo->currentText();
    }
    return w->windowTitle();
}

void widget_set_word_wrap(WidgetHandle const& handle, bool wrap)
{
    if (auto* label = qobject_cast<QLabel*>(handle.get())) {
        label->setWordWrap(wrap);
        label->setAlignment(wrap ? Qt::AlignLeft | Qt::AlignTop : Qt::AlignLeft | Qt::AlignVCenter);
    }
}

bool widget_is_word_wrap(WidgetHandle const& handle)
{
    auto* label = qobject_cast<QLabel*>(handle.get());
    return label && label->wordWrap();
}

void widget_set_placeholder(WidgetHandle const& handle, QString const& text)
{
    QWidget* w = handle.get();
    if (auto* edit = qobject_cast<QLineEdit*>(w)) {
        edit->setPlaceholderText(text);
    } else if (auto* area = qobject_cast<QPlainTextEdit*>(w)) {
        area->setPlaceholderText(text);
    } else if (auto* combo = qobject_cast<QComboBox*>(w)) {
        combo->setPlaceholderText(text);
    }
}

QString widget_placeholder(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* edit = qobject_cast<QLineEdit*>(w)) {
        return edit->placeholderText();
    }
    if (auto* area = qobject_cast<QPlainTextEdit*>(w)) {
        return area->placeholderText();
    }
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        return combo->placeholderText();
    }
    return QString();
}

void widget_set_password_mode(WidgetHandle const& handle, bool enabled)
{
    if (auto* edit = qobject_cast<QLineEdit*>(handle.get())) {
        edit->setEchoMode(enabled ? QLineEdit::Password : QLineEdit::Normal);
    }
}

bool widget_is_password_mode(WidgetHandle const& handle)
{
    auto* edit = qobject_cast<QLineEdit*>(handle.get());
    return edit && edit->echoMode() == QLineEdit::Password;
}

void widget_set_checked(WidgetHandle const& handle, bool checked)
{
    if (auto* button = qobject_cast<QAbstractButton*>(handle.get())) {
        button->setCheckable(true);
        button->setChecked(checked);
    }
}

bool widget_is_checked(WidgetHandle const& handle)
{
    auto* button = qobject_cast<QAbstractButton*>(handle.get());
    return button && button->isChecked();
}

bool widget_click(WidgetHandle const& handle)
{
    auto* button = qobject_cast<QAbstractButton*>(handle.get());
    if (!button) {
        return false;
    }
    button->click();
    return true;
}

void widget_set_read_only(WidgetHandle const& handle, bool readOnly)
{
    QWidget* w = handle.get();
    if (auto* edit = qobject_cast<QLineEdit*>(w)) {
        edit->setReadOnly(readOnly);
    } else if (auto* area = qobject_cast<QPlainTextEdit*>(w)) {
        area->setReadOnly(readOnly);
    } else if (auto* spin = qobject_cast<QAbstractSpinBox*>(w)) {
        spin->setReadOnly(readOnly);
    }
}

bool widget_is_read_only(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* edit = qobject_cast<QLineEdit*>(w)) {
        return edit->isReadOnly();
    }
    if (auto* area = qobject_cast<QPlainTextEdit*>(w)) {
        return area->isReadOnly();
    }
    if (auto* spin = qobject_cast<QAbstractSpinBox*>(w)) {
        return spin->isReadOnly();
    }
    return false;
}

void widget_set_editable(WidgetHandle const& handle, bool editable)
{
    if (auto* combo = qobject_cast<QComboBox*>(handle.get())) {
        combo->setEditable(editable);
    }
}

bool widget_is_editable(WidgetHandle const& handle)
{
    auto* combo = qobject_cast<QComboBox*>(handle.get());
    return combo && combo->isEditable();
}

void widget_add_item(WidgetHandle const& handle, QString const& text)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        combo->addItem(text);
    } else if (auto* list = qobject_cast<QListWidget*>(w)) {
        list->addItem(text);
    }
}

void widget_insert_item(WidgetHandle const& handle, std::int32_t index, QString const& text)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        combo->insertItem(index, text);
    } else if (auto* list = qobject_cast<QListWidget*>(w)) {
        list->insertItem(index, text);
    }
}

void widget_remove_item(WidgetHandle const& handle, std::int32_t index)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        combo->removeItem(index);
    } else if (auto* list = qobject_cast<QListWidget*>(w)) {
        delete list->takeItem(index);
    } else if (auto* tabs = qobject_cast<QTabWidget*>(w)) {
        // Die Seite samt Inhalt gehört uns – removeTab() allein würde sie verwaisen lassen.
        delete tabs->widget(index);
    }
}

void widget_clear_items(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        combo->clear();
    } else if (auto* list = qobject_cast<QListWidget*>(w)) {
        list->clear();
    } else if (auto* tabs = qobject_cast<QTabWidget*>(w)) {
        while (tabs->count() > 0) {
            delete tabs->widget(0);
        }
    }
}

std::int32_t widget_item_count(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        return combo->count();
    }
    if (auto* list = qobject_cast<QListWidget*>(w)) {
        return list->count();
    }
    if (auto* tabs = qobject_cast<QTabWidget*>(w)) {
        return tabs->count();
    }
    return 0;
}

QString widget_item_text(WidgetHandle const& handle, std::int32_t index)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        return combo->itemText(index);
    }
    if (auto* list = qobject_cast<QListWidget*>(w)) {
        QListWidgetItem* item = list->item(index);
        return item ? item->text() : QString();
    }
    if (auto* tabs = qobject_cast<QTabWidget*>(w)) {
        return tabs->tabText(index);
    }
    return QString();
}

void widget_set_item_text(WidgetHandle const& handle, std::int32_t index, QString const& text)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        combo->setItemText(index, text);
    } else if (auto* list = qobject_cast<QListWidget*>(w)) {
        if (QListWidgetItem* item = list->item(index)) {
            item->setText(text);
        }
    } else if (auto* tabs = qobject_cast<QTabWidget*>(w)) {
        tabs->setTabText(index, text);
    }
}

void widget_set_current_index(WidgetHandle const& handle, std::int32_t index)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        combo->setCurrentIndex(index);
    } else if (auto* list = qobject_cast<QListWidget*>(w)) {
        list->setCurrentRow(index);
    } else if (auto* tabs = qobject_cast<QTabWidget*>(w)) {
        tabs->setCurrentIndex(index);
    }
}

std::int32_t widget_current_index(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        return combo->currentIndex();
    }
    if (auto* list = qobject_cast<QListWidget*>(w)) {
        return list->currentRow();
    }
    if (auto* tabs = qobject_cast<QTabWidget*>(w)) {
        return tabs->currentIndex();
    }
    return -1;
}

std::unique_ptr<WidgetHandle> widget_tab_add(WidgetHandle const& handle, QString const& title)
{
    auto* tabs = qobject_cast<QTabWidget*>(handle.get());
    if (!tabs) {
        return std::make_unique<WidgetHandle>(nullptr, false);
    }
    QWidget* page = create(WidgetKind::Container, nullptr);
    tabs->addTab(page, title);
    return std::make_unique<WidgetHandle>(page, true);
}

void widget_set_range(WidgetHandle const& handle, std::int32_t minimum, std::int32_t maximum)
{
    QWidget* w = handle.get();
    if (auto* spin = qobject_cast<QSpinBox*>(w)) {
        spin->setRange(minimum, maximum);
    } else if (auto* slider = qobject_cast<QAbstractSlider*>(w)) {
        slider->setRange(minimum, maximum);
    } else if (auto* bar = qobject_cast<QProgressBar*>(w)) {
        bar->setRange(minimum, maximum);
    }
}

std::int32_t widget_minimum(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* spin = qobject_cast<QSpinBox*>(w)) {
        return spin->minimum();
    }
    if (auto* slider = qobject_cast<QAbstractSlider*>(w)) {
        return slider->minimum();
    }
    if (auto* bar = qobject_cast<QProgressBar*>(w)) {
        return bar->minimum();
    }
    return 0;
}

std::int32_t widget_maximum(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* spin = qobject_cast<QSpinBox*>(w)) {
        return spin->maximum();
    }
    if (auto* slider = qobject_cast<QAbstractSlider*>(w)) {
        return slider->maximum();
    }
    if (auto* bar = qobject_cast<QProgressBar*>(w)) {
        return bar->maximum();
    }
    return 0;
}

void widget_set_value(WidgetHandle const& handle, std::int32_t value)
{
    QWidget* w = handle.get();
    if (auto* spin = qobject_cast<QSpinBox*>(w)) {
        spin->setValue(value);
    } else if (auto* slider = qobject_cast<QAbstractSlider*>(w)) {
        slider->setValue(value);
    } else if (auto* bar = qobject_cast<QProgressBar*>(w)) {
        bar->setValue(value);
    }
}

std::int32_t widget_value(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* spin = qobject_cast<QSpinBox*>(w)) {
        return spin->value();
    }
    if (auto* slider = qobject_cast<QAbstractSlider*>(w)) {
        return slider->value();
    }
    if (auto* bar = qobject_cast<QProgressBar*>(w)) {
        return bar->value();
    }
    return 0;
}

void widget_set_vertical(WidgetHandle const& handle, bool vertical)
{
    Qt::Orientation const orientation = vertical ? Qt::Vertical : Qt::Horizontal;
    QWidget* w = handle.get();
    if (auto* slider = qobject_cast<QAbstractSlider*>(w)) {
        slider->setOrientation(orientation);
    } else if (auto* bar = qobject_cast<QProgressBar*>(w)) {
        bar->setOrientation(orientation);
    }
}

bool widget_is_vertical(WidgetHandle const& handle)
{
    QWidget* w = handle.get();
    if (auto* slider = qobject_cast<QAbstractSlider*>(w)) {
        return slider->orientation() == Qt::Vertical;
    }
    if (auto* bar = qobject_cast<QProgressBar*>(w)) {
        return bar->orientation() == Qt::Vertical;
    }
    return false;
}

bool widget_connect_clicked(WidgetHandle const& handle, rust::Box<RufCallback> callback)
{
    auto* button = qobject_cast<QAbstractButton*>(handle.get());
    if (!button) {
        return false;
    }
    auto cb = share(std::move(callback));
    QObject::connect(button, &QAbstractButton::clicked, button, [cb](bool) {
        invoke_callback(**cb);
    });
    return true;
}

bool widget_connect_toggled(WidgetHandle const& handle, rust::Box<RufBoolCallback> callback)
{
    auto* button = qobject_cast<QAbstractButton*>(handle.get());
    if (!button) {
        return false;
    }
    auto cb = share(std::move(callback));
    QObject::connect(button, &QAbstractButton::toggled, button, [cb](bool checked) {
        invoke_bool_callback(**cb, checked);
    });
    return true;
}

bool widget_connect_text_changed(WidgetHandle const& handle, rust::Box<RufStringCallback> callback)
{
    QWidget* w = handle.get();
    auto cb = share(std::move(callback));
    if (auto* edit = qobject_cast<QLineEdit*>(w)) {
        QObject::connect(edit, &QLineEdit::textChanged, edit, [cb](QString const& text) {
            invoke_string_callback(**cb, text);
        });
        return true;
    }
    if (auto* area = qobject_cast<QPlainTextEdit*>(w)) {
        QObject::connect(area, &QPlainTextEdit::textChanged, area, [cb, area]() {
            invoke_string_callback(**cb, area->toPlainText());
        });
        return true;
    }
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        QObject::connect(combo, &QComboBox::currentTextChanged, combo, [cb](QString const& text) {
            invoke_string_callback(**cb, text);
        });
        return true;
    }
    return false;
}

bool widget_connect_return_pressed(WidgetHandle const& handle, rust::Box<RufCallback> callback)
{
    auto* edit = qobject_cast<QLineEdit*>(handle.get());
    if (!edit) {
        return false;
    }
    auto cb = share(std::move(callback));
    QObject::connect(edit, &QLineEdit::returnPressed, edit, [cb]() {
        invoke_callback(**cb);
    });
    return true;
}

bool widget_connect_value_changed(WidgetHandle const& handle, rust::Box<RufIntCallback> callback)
{
    QWidget* w = handle.get();
    auto cb = share(std::move(callback));
    auto forward = [cb](int value) { invoke_int_callback(**cb, value); };
    if (auto* spin = qobject_cast<QSpinBox*>(w)) {
        QObject::connect(spin, &QSpinBox::valueChanged, spin, forward);
        return true;
    }
    if (auto* slider = qobject_cast<QAbstractSlider*>(w)) {
        QObject::connect(slider, &QAbstractSlider::valueChanged, slider, forward);
        return true;
    }
    if (auto* bar = qobject_cast<QProgressBar*>(w)) {
        QObject::connect(bar, &QProgressBar::valueChanged, bar, forward);
        return true;
    }
    return false;
}

bool widget_connect_current_index_changed(WidgetHandle const& handle, rust::Box<RufIntCallback> callback)
{
    QWidget* w = handle.get();
    auto cb = share(std::move(callback));
    auto forward = [cb](int index) { invoke_int_callback(**cb, index); };
    if (auto* combo = qobject_cast<QComboBox*>(w)) {
        QObject::connect(combo, &QComboBox::currentIndexChanged, combo, forward);
        return true;
    }
    if (auto* list = qobject_cast<QListWidget*>(w)) {
        QObject::connect(list, &QListWidget::currentRowChanged, list, forward);
        return true;
    }
    if (auto* tabs = qobject_cast<QTabWidget*>(w)) {
        QObject::connect(tabs, &QTabWidget::currentChanged, tabs, forward);
        return true;
    }
    return false;
}

bool widget_connect_resized(WidgetHandle const& handle, rust::Box<RufSizeCallback> callback)
{
    QWidget* w = handle.get();
    if (!w) {
        return false;
    }
    w->installEventFilter(new ResizeFilter(w, share(std::move(callback))));
    return true;
}

bool widget_connect_close_requested(WidgetHandle const& handle, rust::Box<RufCloseCallback> callback)
{
    QWidget* w = handle.get();
    if (!w) {
        return false;
    }
    w->installEventFilter(new CloseFilter(w, share(std::move(callback))));
    return true;
}

} // namespace ruf
