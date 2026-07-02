// Qt Widgets implementation of the QuarkGUI renderer. On KDE systems the
// widgets pick up the platform Qt theme (Breeze), matching Kirigami apps.

#include "quark-gui/src/backends/kirigami.rs.h"

#include <QApplication>
#include <QButtonGroup>
#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QRadioButton>
#include <QStyle>
#include <QStyleFactory>
#include <QTimer>
#include <QVBoxLayout>

namespace quark_gui_qt {
namespace {

// Field kinds, mirroring FieldFfi::kind.
constexpr std::uint8_t kFieldText = 0;
constexpr std::uint8_t kFieldList = 1;
constexpr std::uint8_t kFieldRadio = 2;
constexpr std::uint8_t kFieldCombo = 3;
constexpr std::uint8_t kFieldCheck = 4;
constexpr std::uint8_t kFieldPath = 5;
constexpr std::uint8_t kFieldSection = 6;

// Value kinds, mirroring ValueFfi::kind.
constexpr std::uint8_t kValueText = 0;
constexpr std::uint8_t kValueList = 1;
constexpr std::uint8_t kValueIndex = 2;
constexpr std::uint8_t kValueBool = 3;

// Form outcome, mirroring FormResultFfi::outcome.
constexpr std::uint8_t kOutcomeCancel = 0;
constexpr std::uint8_t kOutcomeSubmit = 1;
constexpr std::uint8_t kOutcomeExtra = 2;

// QDialog::exec result codes (0 = rejected/cancel, 1 = accepted/submit).
constexpr int kCodeExtraBase = 100;

QString qs(rust::Str s) { return QString::fromUtf8(s.data(), static_cast<int>(s.size())); }
QString qs(rust::String const &s) { return QString::fromUtf8(s.data(), static_cast<int>(s.size())); }

rust::String rs(QString const &s) {
  QByteArray utf8 = s.toUtf8();
  return rust::String(utf8.constData(), static_cast<std::size_t>(utf8.size()));
}

void ensure_app(bool dark) {
  if (qApp == nullptr) {
    static int argc = 1;
    static char arg0[] = "quark-gui";
    static char *argv[] = {arg0, nullptr};
    new QApplication(argc, argv); // lives for the process
  }
  if (dark) {
    QApplication::setStyle(QStyleFactory::create("Fusion"));
    QPalette palette;
    palette.setColor(QPalette::Window, QColor(45, 45, 45));
    palette.setColor(QPalette::WindowText, QColor(240, 240, 240));
    palette.setColor(QPalette::Base, QColor(30, 30, 30));
    palette.setColor(QPalette::AlternateBase, QColor(45, 45, 45));
    palette.setColor(QPalette::Text, QColor(240, 240, 240));
    palette.setColor(QPalette::Button, QColor(45, 45, 45));
    palette.setColor(QPalette::ButtonText, QColor(240, 240, 240));
    palette.setColor(QPalette::Highlight, QColor(42, 130, 218));
    palette.setColor(QPalette::HighlightedText, Qt::white);
    qApp->setPalette(palette);
  }
}

// One built field, remembered for readback after the dialog closes.
struct BoundField {
  std::uint8_t kind = kFieldText;
  QString id;
  QLineEdit *line = nullptr;
  QPlainTextEdit *text = nullptr;
  QComboBox *combo = nullptr;
  QCheckBox *check = nullptr;
  QButtonGroup *radios = nullptr;
};

QLabel *section_label(QString const &text) {
  auto *label = new QLabel(text);
  QFont font = label->font();
  font.setBold(true);
  label->setFont(font);
  return label;
}

} // namespace

void qt_message(bool is_error, rust::Str title, rust::Str body) {
  ensure_app(false);
  if (is_error) {
    QMessageBox::critical(nullptr, qs(title), qs(body));
  } else {
    QMessageBox::information(nullptr, qs(title), qs(body));
  }
}

FormResultFfi qt_run_form(FormFfi const &form) {
  ensure_app(form.dark);

  QDialog dialog;
  dialog.setWindowTitle(qs(form.title));
  dialog.setMinimumWidth(form.width);

  auto *outer = new QVBoxLayout(&dialog);
  std::vector<BoundField> bound;

  for (FieldFfi const &field : form.fields) {
    BoundField b;
    b.kind = field.kind;
    b.id = qs(field.id);
    switch (field.kind) {
    case kFieldSection: {
      outer->addWidget(section_label(qs(field.label)));
      break;
    }
    case kFieldText:
    case kFieldPath: {
      auto *row = new QHBoxLayout();
      auto *label = new QLabel(qs(field.label));
      label->setMinimumWidth(150);
      row->addWidget(label);
      b.line = new QLineEdit(qs(field.value));
      row->addWidget(b.line, 1);
      if (field.kind == kFieldPath) {
        auto *browse = new QPushButton(QStringLiteral("Browse..."));
        bool directory = field.flag;
        QLineEdit *line = b.line;
        QObject::connect(browse, &QPushButton::clicked, &dialog, [&dialog, line, directory]() {
          QString path = directory
                             ? QFileDialog::getExistingDirectory(&dialog, QStringLiteral("Select folder"))
                             : QFileDialog::getOpenFileName(&dialog, QStringLiteral("Select file"));
          if (!path.isEmpty()) {
            line->setText(path);
          }
        });
        row->addWidget(browse);
      }
      outer->addLayout(row);
      break;
    }
    case kFieldList: {
      outer->addWidget(new QLabel(qs(field.label)));
      b.text = new QPlainTextEdit();
      QStringList lines;
      for (rust::String const &item : field.options) {
        lines.append(qs(item));
      }
      b.text->setPlainText(lines.join(QStringLiteral("\n")));
      b.text->setFixedHeight(110);
      outer->addWidget(b.text);
      break;
    }
    case kFieldCombo: {
      auto *row = new QHBoxLayout();
      auto *label = new QLabel(qs(field.label));
      label->setMinimumWidth(150);
      row->addWidget(label);
      b.combo = new QComboBox();
      for (rust::String const &opt : field.options) {
        b.combo->addItem(qs(opt));
      }
      b.combo->setCurrentIndex(static_cast<int>(field.selected));
      row->addWidget(b.combo, 1);
      outer->addLayout(row);
      break;
    }
    case kFieldCheck: {
      b.check = new QCheckBox(qs(field.label));
      b.check->setChecked(field.flag);
      outer->addWidget(b.check);
      break;
    }
    case kFieldRadio: {
      outer->addWidget(new QLabel(qs(field.label)));
      b.radios = new QButtonGroup(&dialog);
      int i = 0;
      for (rust::String const &opt : field.options) {
        auto *radio = new QRadioButton(qs(opt));
        radio->setChecked(static_cast<std::size_t>(i) == field.selected);
        b.radios->addButton(radio, i);
        outer->addWidget(radio);
        ++i;
      }
      break;
    }
    default:
      break;
    }
    if (field.kind != kFieldSection) {
      bound.push_back(b);
    }
  }

  // Button row, right-aligned: [extras...] [cancel] [submit].
  auto *buttons = new QHBoxLayout();
  buttons->addStretch(1);
  int extra_index = 0;
  for (rust::String const &label : form.extra_labels) {
    auto *button = new QPushButton(qs(label));
    int code = kCodeExtraBase + extra_index;
    QObject::connect(button, &QPushButton::clicked, &dialog, [&dialog, code]() { dialog.done(code); });
    buttons->addWidget(button);
    ++extra_index;
  }
  auto *cancel = new QPushButton(qs(form.cancel_label));
  QObject::connect(cancel, &QPushButton::clicked, &dialog, &QDialog::reject);
  buttons->addWidget(cancel);
  auto *submit = new QPushButton(qs(form.submit_label));
  submit->setDefault(true);
  QObject::connect(submit, &QPushButton::clicked, &dialog, &QDialog::accept);
  buttons->addWidget(submit);
  outer->addLayout(buttons);

  int code = dialog.exec();

  FormResultFfi result{};
  result.outcome = code == QDialog::Accepted            ? kOutcomeSubmit
                   : code >= kCodeExtraBase             ? kOutcomeExtra
                                                        : kOutcomeCancel;
  result.extra_index = code >= kCodeExtraBase ? static_cast<std::size_t>(code - kCodeExtraBase) : 0;

  for (BoundField const &b : bound) {
    ValueFfi value{};
    value.id = rs(b.id);
    switch (b.kind) {
    case kFieldText:
    case kFieldPath:
      value.kind = kValueText;
      value.text = rs(b.line->text());
      break;
    case kFieldList: {
      value.kind = kValueList;
      QStringList lines = b.text->toPlainText().split(QLatin1Char('\n'));
      for (QString const &line : lines) {
        QString trimmed = line.trimmed();
        if (!trimmed.isEmpty()) {
          value.list.push_back(rs(trimmed));
        }
      }
      break;
    }
    case kFieldCombo:
      value.kind = kValueIndex;
      value.index = static_cast<std::size_t>(std::max(0, b.combo->currentIndex()));
      break;
    case kFieldCheck:
      value.kind = kValueBool;
      value.flag = b.check->isChecked();
      break;
    case kFieldRadio:
      value.kind = kValueIndex;
      value.index = static_cast<std::size_t>(std::max(0, b.radios->checkedId()));
      break;
    default:
      continue;
    }
    result.values.push_back(std::move(value));
  }
  return result;
}

std::int32_t qt_run_progress(ProgressFfi const &spec, rust::Box<ProgressSource> source) {
  ensure_app(spec.dark);

  QDialog dialog;
  dialog.setWindowTitle(qs(spec.title));
  dialog.setMinimumWidth(460);

  auto *outer = new QVBoxLayout(&dialog);
  auto *queue = new QLabel(QString());
  auto *bar = new QProgressBar();
  bar->setRange(0, 1000);
  auto *status = new QLabel(qs(spec.initial_status));
  auto *eta = new QLabel(QString());
  auto *buttons = new QHBoxLayout();
  buttons->addStretch(1);
  auto *cancel = new QPushButton(QStringLiteral("Cancel"));
  QObject::connect(cancel, &QPushButton::clicked, &dialog, &QDialog::reject);
  buttons->addWidget(cancel);

  outer->addWidget(queue);
  outer->addWidget(bar);
  outer->addWidget(status);
  outer->addWidget(eta);
  outer->addLayout(buttons);

  bool finished = false;
  std::int32_t exit_code = 1;

  QTimer timer;
  timer.setInterval(50);
  ProgressSource const &src = *source;
  QObject::connect(&timer, &QTimer::timeout, &dialog,
                   [&src, &dialog, &finished, &exit_code, queue, bar, status, eta]() {
                     while (true) {
                       PollFfi update = src.poll();
                       // kind: 0 empty, 1 percent, 2 status, 3 eta, 4 queue, 5 done.
                       if (update.kind == 0) {
                         return;
                       }
                       switch (update.kind) {
                       case 1:
                         bar->setValue(static_cast<int>(update.number * 10.0));
                         break;
                       case 2:
                         status->setText(qs(update.text));
                         break;
                       case 3:
                         eta->setText(qs(update.text));
                         break;
                       case 4:
                         queue->setText(qs(update.text));
                         break;
                       case 5:
                         finished = true;
                         exit_code = update.code;
                         dialog.accept();
                         return;
                       default:
                         break;
                       }
                     }
                   });
  timer.start();

  dialog.exec();
  timer.stop();

  if (!finished) {
    // Cancelled before completion.
    source->request_cancel();
    return 1;
  }
  return exit_code;
}

} // namespace quark_gui_qt
