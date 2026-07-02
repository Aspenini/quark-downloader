// C++ side of the Qt backend bridge. The shared struct definitions live in the
// cxx-generated header (kirigami.rs.h); they are forward-declared here so this
// header can be include!d from the bridge without a cycle.
#pragma once

#include <cstdint>

#include "rust/cxx.h"

namespace quark_gui_qt {

struct FormFfi;
struct FormResultFfi;
struct ProgressFfi;
struct ProgressSource;

void qt_message(bool is_error, rust::Str title, rust::Str body);
FormResultFfi qt_run_form(FormFfi const &form);
std::int32_t qt_run_progress(ProgressFfi const &spec, rust::Box<ProgressSource> source);

} // namespace quark_gui_qt
