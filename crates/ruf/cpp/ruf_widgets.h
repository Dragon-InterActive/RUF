// RUF – C++-Seite der System-Ebene.
//
// Dünne, absichtlich "dumme" Schicht über QtWidgets. Alle Logik liegt in Rust;
// hier wird nur erzeugt, weitergereicht und verbunden.
#pragma once

#include <QtCore/QPointer>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtCore/QTimer>
#include <QtCore/QtGlobal>
#include <QtWidgets/QApplication>
#include <QtWidgets/QWidget>

#include <memory>
#include <string>
#include <vector>

#include "rust/cxx.h"

static_assert(QT_VERSION >= QT_VERSION_CHECK(6, 0, 0), "RUF benötigt Qt >= 6");

namespace ruf {

// In Rust definierte Typen (siehe src/ffi.rs).
struct RufCallback;
struct RufBoolCallback;
struct RufStringCallback;
struct RufIntCallback;
struct RufSizeCallback;
struct RufSendCallback;
struct RufCloseCallback;
class WidgetHandle;
enum class WidgetKind : ::std::uint8_t;
enum class ColorScheme : ::std::uint8_t;

// ---------------------------------------------------------------------------
// Anwendung
// ---------------------------------------------------------------------------

// Besitzt QApplication samt argc/argv, die Qt für die gesamte Lebensdauer braucht.
class Application {
public:
    explicit Application(rust::Vec<rust::String> const& args);
    ~Application();

    Application(Application const&) = delete;
    Application& operator=(Application const&) = delete;

private:
    int m_argc;
    std::vector<std::string> m_args;
    std::vector<char*> m_argv;
    std::unique_ptr<QApplication> m_app;
};

std::unique_ptr<Application> application_new(rust::Vec<rust::String> const& args);
bool application_exists();
std::int32_t application_exec();
void application_quit();
void application_process_events();
void application_set_name(QString const& name);
QString application_name();

ColorScheme application_color_scheme();
bool application_set_color_scheme(ColorScheme scheme);

// Führt den Callback im GUI-Thread aus. Darf aus jedem Thread aufgerufen werden;
// liefert false (und verwirft den Callback), wenn keine Anwendung existiert.
bool post_to_gui(rust::Box<RufSendCallback> callback);

// Meldungs- und Rückfragedialoge (modal). `parent` darf ein leeres Handle sein.
// kind: 0 = Information, 1 = Warnung, 2 = Fehler.
void dialog_message(WidgetHandle const& parent, std::int32_t kind, QString const& title, QString const& text);
bool dialog_confirm(WidgetHandle const& parent, QString const& title, QString const& text);

std::int32_t qt_compile_version();
QString qt_runtime_version();

// ---------------------------------------------------------------------------
// Timer
// ---------------------------------------------------------------------------

// Besitzt einen QTimer; lebt und feuert im GUI-Thread.
class TimerHandle {
public:
    TimerHandle();
    ~TimerHandle();

    TimerHandle(TimerHandle const&) = delete;
    TimerHandle& operator=(TimerHandle const&) = delete;

    QTimer* get() const { return m_timer.get(); }

private:
    std::unique_ptr<QTimer> m_timer;
};

std::unique_ptr<TimerHandle> timer_new();
void timer_connect_timeout(TimerHandle const& timer, rust::Box<RufCallback> callback);
void timer_start(TimerHandle const& timer, std::int32_t msec);
void timer_stop(TimerHandle const& timer);
bool timer_is_active(TimerHandle const& timer);
void timer_set_single_shot(TimerHandle const& timer, bool singleShot);
bool timer_is_single_shot(TimerHandle const& timer);
std::int32_t timer_interval(TimerHandle const& timer);
void timer_single_shot(std::int32_t msec, rust::Box<RufCallback> callback);

// ---------------------------------------------------------------------------
// Widget-Handle
// ---------------------------------------------------------------------------

// Sicherer Verweis auf ein QWidget.
//
// * QPointer wird automatisch null, sobald Qt das Widget zerstört (z. B. weil
//   das Elternfenster geschlossen und gelöscht wurde). Alle Operationen sind
//   dann No-Ops – es gibt keine hängenden Zeiger auf der Rust-Seite.
// * Besitzt das Widget beim Zerstören des Handles kein Elternteil, gehört es
//   uns und wird gelöscht. Kinder gehören dagegen ihrem Elternteil (Qt-Modell).
class WidgetHandle {
public:
    explicit WidgetHandle(QWidget* widget, bool owned);
    ~WidgetHandle();

    WidgetHandle(WidgetHandle const&) = delete;
    WidgetHandle& operator=(WidgetHandle const&) = delete;

    QWidget* get() const { return m_widget.data(); }

private:
    QPointer<QWidget> m_widget;
    bool m_owned;
};

std::unique_ptr<WidgetHandle> widget_create(WidgetKind kind);
std::unique_ptr<WidgetHandle> widget_create_child(WidgetKind kind, WidgetHandle const& parent);
std::unique_ptr<WidgetHandle> widget_wrap(QWidget* widget);

QWidget* widget_raw(WidgetHandle const& handle);
bool widget_is_alive(WidgetHandle const& handle);
WidgetKind widget_kind(WidgetHandle const& handle);
QString widget_class_name(WidgetHandle const& handle);
void widget_set_parent(WidgetHandle const& handle, WidgetHandle const& parent);

void widget_set_geometry(WidgetHandle const& handle, QRect const& rect);
QRect widget_geometry(WidgetHandle const& handle);
void widget_set_minimum_size(WidgetHandle const& handle, std::int32_t width, std::int32_t height);
void widget_adjust_size(WidgetHandle const& handle);

void widget_set_visible(WidgetHandle const& handle, bool visible);
bool widget_is_visible(WidgetHandle const& handle);
bool widget_is_hidden(WidgetHandle const& handle);
void widget_set_enabled(WidgetHandle const& handle, bool enabled);
bool widget_is_enabled(WidgetHandle const& handle);
void widget_set_tool_tip(WidgetHandle const& handle, QString const& text);
QString widget_tool_tip(WidgetHandle const& handle);
void widget_set_style_sheet(WidgetHandle const& handle, QString const& css);
QString widget_style_sheet(WidgetHandle const& handle);
void widget_set_object_name(WidgetHandle const& handle, QString const& name);
QString widget_object_name(WidgetHandle const& handle);
void widget_close(WidgetHandle const& handle);
bool widget_save_screenshot(WidgetHandle const& handle, QString const& path);
// Löscht das Widget samt Kindern, sobald der Event-Loop wieder dran ist.
void widget_delete_later(WidgetHandle const& handle);
// Tastenkürzel (z. B. "Ctrl+S"), aktiv solange das Fenster des Widgets aktiv ist.
bool widget_add_shortcut(WidgetHandle const& handle, QString const& sequence, rust::Box<RufCallback> callback);
// Inhalt einer Scrollfläche (leeres Handle bei anderen Widgets).
std::unique_ptr<WidgetHandle> widget_scroll_content(WidgetHandle const& handle);

// Textbereich als Code-Editor.
void widget_set_monospace(WidgetHandle const& handle, bool monospace);
bool widget_is_monospace(WidgetHandle const& handle);
bool widget_add_highlight_rule(WidgetHandle const& handle, QString const& pattern, QString const& color, bool bold);
void widget_clear_highlight_rules(WidgetHandle const& handle);
void widget_set_marked_line(WidgetHandle const& handle, std::int32_t line, QString const& color);
void widget_goto_line(WidgetHandle const& handle, std::int32_t line);

// Text: Fenstertitel, Button-, Label- oder Eingabefeldtext – je nach Widget-Art.
void widget_set_text(WidgetHandle const& handle, QString const& text);
QString widget_text(WidgetHandle const& handle);
// Zeilenumbruch bei Labels.
void widget_set_word_wrap(WidgetHandle const& handle, bool wrap);
bool widget_is_word_wrap(WidgetHandle const& handle);
void widget_set_placeholder(WidgetHandle const& handle, QString const& text);
QString widget_placeholder(WidgetHandle const& handle);
void widget_set_password_mode(WidgetHandle const& handle, bool enabled);
bool widget_is_password_mode(WidgetHandle const& handle);
void widget_set_checked(WidgetHandle const& handle, bool checked);
bool widget_is_checked(WidgetHandle const& handle);
bool widget_click(WidgetHandle const& handle);
void widget_set_read_only(WidgetHandle const& handle, bool readOnly);
bool widget_is_read_only(WidgetHandle const& handle);
void widget_set_editable(WidgetHandle const& handle, bool editable);
bool widget_is_editable(WidgetHandle const& handle);

// Einträge: Combobox, Liste und – für Titel, Anzahl, Entfernen – Registerkarten.
void widget_add_item(WidgetHandle const& handle, QString const& text);
void widget_insert_item(WidgetHandle const& handle, std::int32_t index, QString const& text);
void widget_remove_item(WidgetHandle const& handle, std::int32_t index);
void widget_clear_items(WidgetHandle const& handle);
std::int32_t widget_item_count(WidgetHandle const& handle);
QString widget_item_text(WidgetHandle const& handle, std::int32_t index);
void widget_set_item_text(WidgetHandle const& handle, std::int32_t index, QString const& text);
void widget_set_current_index(WidgetHandle const& handle, std::int32_t index);
std::int32_t widget_current_index(WidgetHandle const& handle);
// Neue Seite einer Registerkarte; bei anderen Widgets ein leeres Handle.
std::unique_ptr<WidgetHandle> widget_tab_add(WidgetHandle const& handle, QString const& title);

// Wertebereich: Spinbox, Schieberegler, Fortschrittsbalken.
void widget_set_range(WidgetHandle const& handle, std::int32_t minimum, std::int32_t maximum);
std::int32_t widget_minimum(WidgetHandle const& handle);
std::int32_t widget_maximum(WidgetHandle const& handle);
void widget_set_value(WidgetHandle const& handle, std::int32_t value);
std::int32_t widget_value(WidgetHandle const& handle);
void widget_set_vertical(WidgetHandle const& handle, bool vertical);
bool widget_is_vertical(WidgetHandle const& handle);

// Signale -> Rust-Closures. Liefert false, wenn das Widget das Signal nicht kennt.
bool widget_connect_clicked(WidgetHandle const& handle, rust::Box<RufCallback> callback);
bool widget_connect_toggled(WidgetHandle const& handle, rust::Box<RufBoolCallback> callback);
bool widget_connect_text_changed(WidgetHandle const& handle, rust::Box<RufStringCallback> callback);
bool widget_connect_return_pressed(WidgetHandle const& handle, rust::Box<RufCallback> callback);
bool widget_connect_value_changed(WidgetHandle const& handle, rust::Box<RufIntCallback> callback);
bool widget_connect_current_index_changed(WidgetHandle const& handle, rust::Box<RufIntCallback> callback);
// Größenänderung (neue Breite, Höhe) – funktioniert bei jedem Widget.
bool widget_connect_resized(WidgetHandle const& handle, rust::Box<RufSizeCallback> callback);
// Schließen-Anfrage eines Fensters; liefert der Callback false, bleibt es offen.
bool widget_connect_close_requested(WidgetHandle const& handle, rust::Box<RufCloseCallback> callback);

} // namespace ruf
