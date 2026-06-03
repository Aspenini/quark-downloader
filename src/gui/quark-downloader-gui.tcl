#!/usr/bin/env wish
# Quark Downloader — Tk GUI (spawned by quark-downloader-gui via wish)

package require Tk

set AUDIO_FORMATS {original mp3 m4a flac wav opus vorbis}
set VIDEO_FORMATS {original mp4 mkv webm}

proc show_message {kind title body} {
    set icon ok
    if {$kind eq "error"} {
        set icon error
    }
    tk_messageBox -title $title -message $body -type ok -icon $icon
    exit 0
}

if {[llength $argv] > 0 && [lindex $argv 0] eq "--message"} {
    if {[llength $argv] < 4} {
        puts stderr "usage: $argv0 --message <ok|error> <title> <body>"
        exit 2
    }
    show_message [lindex $argv 1] [lindex $argv 2] [join [lrange $argv 3 end]]
}

set default_dir [file normalize "~/Downloads"]
if {[llength $argv] > 0} {
    set default_dir [file normalize [lindex $argv 0]]
}

set ::media_type video
set ::confirmed 0

proc set_formats {} {
    global media_type
    if {$media_type eq "audio"} {
        set values $::AUDIO_FORMATS
    } else {
        set values $::VIDEO_FORMATS
    }
    .format_combo configure -values $values
    .format_combo current 0
}

proc on_cancel {} {
    destroy .
    exit 1
}

proc on_download {} {
    global media_type

    set url [string trim [.url_entry get]]
    if {$url eq ""} {
        tk_messageBox -title "Quark Downloader" -message "Please enter a video URL." \
            -type ok -icon error
        return
    }

    set output [string trim [.output_entry get]]
    if {$output eq ""} {
        tk_messageBox -title "Quark Downloader" -message "Please choose an output folder." \
            -type ok -icon error
        return
    }

    set format [.format_combo get]
    if {$format eq ""} {
        set format original
    }

    puts $url
    puts $media_type
    puts $format
    puts $output
    flush stdout
    destroy .
    exit 0
}

proc on_browse {} {
    global default_dir
    set initial [string trim [.output_entry get]]
    if {$initial eq ""} {
        set initial $default_dir
    }
    set chosen [tk_chooseDirectory -mustexist 1 -initialdir $initial \
        -title "Select output folder"]
    if {$chosen ne ""} {
        .output_entry delete 0 end
        .output_entry insert 0 $chosen
    }
}

proc on_type_change {} {
    global media_type type_var
    set media_type $type_var
    set_formats
}

wm title . "Quark Downloader"
wm resizable . 0 0

ttk::label .url_lbl -text "Video URL:"
ttk::entry .url_entry -width 42
grid .url_lbl -row 0 -column 0 -columnspan 3 -sticky w -padx 10 -pady {10 2}
grid .url_entry -row 1 -column 0 -columnspan 3 -sticky ew -padx 10 -pady {0 8}

set type_var video
ttk::radiobutton .rb_video -text "Video" -variable type_var -value video \
    -command on_type_change
ttk::radiobutton .rb_audio -text "Audio" -variable type_var -value audio \
    -command on_type_change
grid .rb_video -row 2 -column 0 -sticky w -padx {10 0}
grid .rb_audio -row 2 -column 1 -sticky w -padx 5

ttk::label .fmt_lbl -text "Format:"
ttk::combobox .format_combo -state readonly -width 18
grid .fmt_lbl -row 3 -column 0 -sticky w -padx 10 -pady {8 2}
grid .format_combo -row 4 -column 0 -columnspan 2 -sticky w -padx 10 -pady {0 8}

ttk::label .out_lbl -text "Output folder:"
ttk::entry .output_entry -width 32
ttk::button .browse_btn -text "Browse..." -command on_browse
grid .out_lbl -row 5 -column 0 -columnspan 3 -sticky w -padx 10 -pady {0 2}
grid .output_entry -row 6 -column 0 -columnspan 2 -sticky ew -padx {10 0}
grid .browse_btn -row 6 -column 2 -sticky e -padx {4 10}

.output_entry insert 0 $default_dir

ttk::button .dl_btn -text "Download" -command on_download -default active
ttk::button .cancel_btn -text "Cancel" -command on_cancel
grid .dl_btn -row 7 -column 1 -sticky e -padx 5 -pady 12
grid .cancel_btn -row 7 -column 2 -sticky e -padx {0 10} -pady 12

grid columnconfigure . 0 -weight 1
grid columnconfigure . 1 -weight 0

wm protocol . WM_DELETE_WINDOW on_cancel
set_formats

bind . <Return> on_download
bind . <Escape> on_cancel
