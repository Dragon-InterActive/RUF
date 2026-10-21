#include "ruf/cpp/ruf_widgets.h"

// Von cxx-qt generiert: vollständige Definitionen der gemeinsamen Enums und
// die Rust-Funktionen zum Aufruf der Callbacks.
#include "ruf/src/ffi.cxxqt.h"

#include <QtGui/QGuiApplication>
#include <QtGui/QPixmap>
#include <QtGui/QStyleHints>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>

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
    return std::make_unique<WidgetHandle>(create(kind, parent.get()), true);
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
    if (qobject_cast<QLabel*>(w)) {
        return WidgetKind::Label;
    }
    if (qobject_cast<QLineEdit*>(w)) {
        return WidgetKind::LineEdit;
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
    return w->windowTitle();
}

void widget_set_placeholder(WidgetHandle const& handle, QString const& text)
{
    if (auto* edit = qobject_cast<QLineEdit*>(handle.get())) {
        edit->setPlaceholderText(text);
    }
}

QString widget_placeholder(WidgetHandle const& handle)
{
    auto* edit = qobject_cast<QLineEdit*>(handle.get());
    return edit ? edit->placeholderText() : QString();
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
    auto* edit = qobject_cast<QLineEdit*>(handle.get());
    if (!edit) {
        return false;
    }
    auto cb = share(std::move(callback));
    QObject::connect(edit, &QLineEdit::textChanged, edit, [cb](QString const& text) {
        invoke_string_callback(**cb, text);
    });
    return true;
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

} // namespace ruf
