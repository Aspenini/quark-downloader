#!/usr/bin/env wish
# Quark Downloader - Tk GUI (spawned by quark-downloader-gui via wish)

package require Tk

set ::APP_NAME "Quark Downloader"
if {[info exists ::env(QUARK_VERSION)] && [string length $::env(QUARK_VERSION)] > 0} {
    set ::APP_VERSION $::env(QUARK_VERSION)
} else {
    set ::APP_VERSION ""
}

proc app_window_title {} {
    if {$::APP_VERSION ne ""} {
        return "$::APP_NAME $::APP_VERSION"
    }
    return $::APP_NAME
}

proc app_settings_window_title {} {
    return "[app_window_title] Settings"
}

set AUDIO_FORMATS {original mp3 m4a flac wav opus vorbis}
set VIDEO_FORMATS {original mp4 mkv webm}

proc show_message {kind title body} {
    wm withdraw .
    update idletasks
    set icon info
    if {$kind eq "error"} {
        set icon error
    }
    tk_messageBox -parent . -title $title -message $body -type ok -icon $icon
    exit 0
}

if {[llength $argv] > 0 && [lindex $argv 0] eq "--message"} {
    if {[llength $argv] < 4} {
        puts stderr "usage: $argv0 --message <ok|error> <title> <body>"
        exit 2
    }
    show_message [lindex $argv 1] [lindex $argv 2] [join [lrange $argv 3 end]]
} elseif {[llength $argv] > 0 && [lindex $argv 0] eq "--session"} {
    set default_dir [file normalize "~/Downloads"]
    set current_download_dir "~/Downloads"
    set current_ytdlp auto
    set current_ffmpeg auto
    set current_gui_mode progress
    set current_logs true

    if {[llength $argv] > 1} { set default_dir [lindex $argv 1] }
    if {[llength $argv] > 2} { set current_download_dir [lindex $argv 2] }
    if {[llength $argv] > 3} { set current_ytdlp [lindex $argv 3] }
    if {[llength $argv] > 4} { set current_ffmpeg [lindex $argv 4] }
    if {[llength $argv] > 5} { set current_gui_mode [lindex $argv 5] }
    if {[llength $argv] > 6} { set current_logs [lindex $argv 6] }

    proc session_set_combo_value {widget value values} {
        $widget configure -values $values
        set idx [lsearch -exact $values $value]
        if {$idx < 0} {
            set idx 0
        }
        $widget current $idx
    }

    proc session_bool_value {value} {
        set lowered [string tolower $value]
        return [expr {$lowered eq "true" || $lowered eq "1" || $lowered eq "yes" || $lowered eq "on"}]
    }

    proc session_normalize_dir {path} {
        if {[catch {file normalize $path} normalized]} {
            return $path
        }
        return $normalized
    }

    set ::session_default_dir [session_normalize_dir $default_dir]
    set ::session_media_type video
    set ::session_type_var video
    set ::session_settings_saved 0
    set ::session_download_dir $current_download_dir
    set ::session_ytdlp $current_ytdlp
    set ::session_ffmpeg $current_ffmpeg
    set ::session_gui_mode $current_gui_mode
    set ::session_logs [session_bool_value $current_logs]
    set ::session_logs_var $::session_logs

    proc session_set_formats {} {
        if {$::session_media_type eq "audio"} {
            set values $::AUDIO_FORMATS
        } else {
            set values $::VIDEO_FORMATS
        }
        .main.format_combo configure -values $values
        .main.format_combo current 0
    }

    proc session_bind_main_keys {} {
        bind . <Return> session_on_download
        bind . <Escape> session_emit_cancel
    }

    proc session_bind_settings_keys {} {
        bind . <Return> session_on_settings_save
        bind . <Escape> session_show_main
    }

    proc session_show_main {} {
        wm title . [app_window_title]
        catch {grid remove .settings}
        grid .main -row 0 -column 0 -sticky nsew
        session_bind_main_keys
        focus .main.url_entry
    }

    proc session_show_settings {} {
        wm title . [app_settings_window_title]
        catch {grid remove .main}

        .settings.download_entry delete 0 end
        .settings.download_entry insert 0 $::session_download_dir
        session_set_combo_value .settings.ytdlp_combo $::session_ytdlp {auto path bundled}
        session_set_combo_value .settings.ffmpeg_combo $::session_ffmpeg {auto path bundled}
        session_set_combo_value .settings.mode_combo $::session_gui_mode {progress external_cli}
        set ::session_logs_var $::session_logs

        grid .settings -row 0 -column 0 -sticky nsew
        session_bind_settings_keys
        focus .settings.download_entry
    }

    proc session_emit_settings {} {
        puts "__SETTINGS__"
        puts $::session_download_dir
        puts $::session_ytdlp
        puts $::session_ffmpeg
        puts $::session_gui_mode
        if {$::session_logs} {
            puts "true"
        } else {
            puts "false"
        }
    }

    proc session_emit_download {url media_type format output} {
        puts "__SESSION__"
        if {$::session_settings_saved} {
            session_emit_settings
        }
        puts "__DOWNLOAD__"
        puts $url
        puts $media_type
        puts $format
        puts $output
        flush stdout
        destroy .
        exit 0
    }

    proc session_emit_cancel {} {
        puts "__SESSION__"
        if {$::session_settings_saved} {
            session_emit_settings
        }
        puts "__CANCEL__"
        flush stdout
        destroy .
        exit 0
    }

    proc session_on_type_change {} {
        set ::session_media_type $::session_type_var
        session_set_formats
    }

    proc session_on_browse {} {
        set initial [string trim [.main.output_entry get]]
        if {$initial eq ""} {
            set initial $::session_default_dir
        }
        set chosen [tk_chooseDirectory -parent . -mustexist 1 \
            -initialdir [session_normalize_dir $initial] -title "Select output folder"]
        if {$chosen ne ""} {
            .main.output_entry delete 0 end
            .main.output_entry insert 0 $chosen
        }
    }

    proc session_on_settings_browse {} {
        set initial [string trim [.settings.download_entry get]]
        if {$initial eq ""} {
            set initial [file normalize "~/Downloads"]
        }
        set chosen [tk_chooseDirectory -parent . -mustexist 1 \
            -initialdir [session_normalize_dir $initial] \
            -title "Select default download folder"]
        if {$chosen ne ""} {
            .settings.download_entry delete 0 end
            .settings.download_entry insert 0 $chosen
        }
    }

    proc session_on_settings_save {} {
        set download_dir [string trim [.settings.download_entry get]]
        if {$download_dir eq ""} {
            tk_messageBox -parent . -title "Quark Downloader" \
                -message "Please choose a default download folder." -type ok -icon error
            return
        }

        set previous_default $::session_default_dir
        set normalized_download_dir [session_normalize_dir $download_dir]
        set current_output [string trim [.main.output_entry get]]

        set ::session_download_dir $download_dir
        set ::session_ytdlp [.settings.ytdlp_combo get]
        set ::session_ffmpeg [.settings.ffmpeg_combo get]
        set ::session_gui_mode [.settings.mode_combo get]
        set ::session_logs $::session_logs_var
        set ::session_default_dir $normalized_download_dir
        set ::session_settings_saved 1

        if {$current_output eq "" || $current_output eq $previous_default} {
            .main.output_entry delete 0 end
            .main.output_entry insert 0 $normalized_download_dir
        }

        session_show_main
    }

    proc session_on_download {} {
        set url [string trim [.main.url_entry get]]
        if {$url eq ""} {
            tk_messageBox -parent . -title "Quark Downloader" \
                -message "Please enter a video URL." -type ok -icon error
            return
        }

        set output [string trim [.main.output_entry get]]
        if {$output eq ""} {
            tk_messageBox -parent . -title "Quark Downloader" \
                -message "Please choose an output folder." -type ok -icon error
            return
        }

        set format [.main.format_combo get]
        if {$format eq ""} {
            set format original
        }

        session_emit_download $url $::session_media_type $format $output
    }

    wm title . [app_window_title]
    wm resizable . 0 0

    ttk::frame .main
    ttk::frame .settings
    grid columnconfigure . 0 -weight 1
    grid rowconfigure . 0 -weight 1

    ttk::label .main.url_lbl -text "Video URL:"
    ttk::entry .main.url_entry -width 42
    grid .main.url_lbl -row 0 -column 0 -columnspan 3 -sticky w -padx 10 -pady {10 2}
    grid .main.url_entry -row 1 -column 0 -columnspan 3 -sticky ew -padx 10 -pady {0 8}

    ttk::radiobutton .main.rb_video -text "Video" -variable ::session_type_var -value video \
        -command session_on_type_change
    ttk::radiobutton .main.rb_audio -text "Audio" -variable ::session_type_var -value audio \
        -command session_on_type_change
    grid .main.rb_video -row 2 -column 0 -sticky w -padx {10 0}
    grid .main.rb_audio -row 2 -column 1 -sticky w -padx 5

    ttk::label .main.fmt_lbl -text "Format:"
    ttk::combobox .main.format_combo -state readonly -width 18
    grid .main.fmt_lbl -row 3 -column 0 -sticky w -padx 10 -pady {8 2}
    grid .main.format_combo -row 4 -column 0 -columnspan 2 -sticky w -padx 10 -pady {0 8}

    ttk::label .main.out_lbl -text "Output folder:"
    ttk::entry .main.output_entry -width 32
    ttk::button .main.browse_btn -text "Browse..." -command session_on_browse
    grid .main.out_lbl -row 5 -column 0 -columnspan 3 -sticky w -padx 10 -pady {0 2}
    grid .main.output_entry -row 6 -column 0 -columnspan 2 -sticky ew -padx {10 0}
    grid .main.browse_btn -row 6 -column 2 -sticky e -padx {4 10}

    .main.output_entry insert 0 $::session_default_dir

    ttk::button .main.settings_btn -text "\u2699" -width 3 -command session_show_settings
    ttk::button .main.dl_btn -text "Download" -command session_on_download -default active
    ttk::button .main.cancel_btn -text "Cancel" -command session_emit_cancel
    grid .main.settings_btn -row 7 -column 0 -sticky w -padx 10 -pady 12
    grid .main.dl_btn -row 7 -column 1 -sticky e -padx 5 -pady 12
    grid .main.cancel_btn -row 7 -column 2 -sticky e -padx {0 10} -pady 12

    grid columnconfigure .main 0 -weight 1
    grid columnconfigure .main 1 -weight 0

    ttk::label .settings.download_lbl -text "Default folder:"
    ttk::entry .settings.download_entry -width 38
    ttk::button .settings.download_browse_btn -text "Browse..." \
        -command session_on_settings_browse
    grid .settings.download_lbl -row 0 -column 0 -columnspan 3 -sticky w -padx 10 -pady {10 2}
    grid .settings.download_entry -row 1 -column 0 -columnspan 2 -sticky ew -padx {10 0}
    grid .settings.download_browse_btn -row 1 -column 2 -sticky e -padx {4 10}

    ttk::label .settings.ytdlp_lbl -text "yt-dlp:"
    ttk::combobox .settings.ytdlp_combo -state readonly -width 16
    ttk::label .settings.ffmpeg_lbl -text "ffmpeg:"
    ttk::combobox .settings.ffmpeg_combo -state readonly -width 16
    grid .settings.ytdlp_lbl -row 2 -column 0 -sticky w -padx 10 -pady {10 2}
    grid .settings.ytdlp_combo -row 3 -column 0 -sticky w -padx 10
    grid .settings.ffmpeg_lbl -row 2 -column 1 -sticky w -padx 10 -pady {10 2}
    grid .settings.ffmpeg_combo -row 3 -column 1 -sticky w -padx 10

    ttk::label .settings.mode_lbl -text "GUI download:"
    ttk::combobox .settings.mode_combo -state readonly -width 16
    ttk::checkbutton .settings.logs_check -text "Create download logs" \
        -variable ::session_logs_var
    grid .settings.mode_lbl -row 4 -column 0 -sticky w -padx 10 -pady {10 2}
    grid .settings.mode_combo -row 5 -column 0 -sticky w -padx 10
    grid .settings.logs_check -row 5 -column 1 -columnspan 2 -sticky w -padx 10

    ttk::button .settings.save_btn -text "Save" -command session_on_settings_save -default active
    ttk::button .settings.cancel_btn -text "Cancel" -command session_show_main
    grid .settings.save_btn -row 6 -column 1 -sticky e -padx 5 -pady 12
    grid .settings.cancel_btn -row 6 -column 2 -sticky e -padx {0 10} -pady 12

    grid columnconfigure .settings 0 -weight 1
    grid columnconfigure .settings 1 -weight 1

    wm protocol . WM_DELETE_WINDOW session_emit_cancel
    session_set_formats
    session_show_main
} elseif {[llength $argv] > 0 && [lindex $argv 0] eq "--settings"} {
    set current_download_dir "~/Downloads"
    set current_ytdlp auto
    set current_ffmpeg auto
    set current_gui_mode progress
    set current_logs true

    if {[llength $argv] > 1} { set current_download_dir [lindex $argv 1] }
    if {[llength $argv] > 2} { set current_ytdlp [lindex $argv 2] }
    if {[llength $argv] > 3} { set current_ffmpeg [lindex $argv 3] }
    if {[llength $argv] > 4} { set current_gui_mode [lindex $argv 4] }
    if {[llength $argv] > 5} { set current_logs [lindex $argv 5] }

    proc set_combo_value {widget value values} {
        $widget configure -values $values
        set idx [lsearch -exact $values $value]
        if {$idx < 0} {
            set idx 0
        }
        $widget current $idx
    }

    proc bool_value {value} {
        set lowered [string tolower $value]
        return [expr {$lowered eq "true" || $lowered eq "1" || $lowered eq "yes" || $lowered eq "on"}]
    }

    proc on_settings_browse {} {
        set initial [string trim [.download_entry get]]
        if {$initial eq ""} {
            set initial [file normalize "~/Downloads"]
        }
        set chosen [tk_chooseDirectory -mustexist 1 -initialdir $initial \
            -title "Select default download folder"]
        if {$chosen ne ""} {
            .download_entry delete 0 end
            .download_entry insert 0 $chosen
        }
    }

    proc on_settings_save {} {
        set download_dir [string trim [.download_entry get]]
        if {$download_dir eq ""} {
            tk_messageBox -title "Quark Downloader" -message "Please choose a default download folder." \
                -type ok -icon error
            return
        }

        puts "__SETTINGS__"
        puts $download_dir
        puts [.ytdlp_combo get]
        puts [.ffmpeg_combo get]
        puts [.mode_combo get]
        if {$::logs_var} {
            puts "true"
        } else {
            puts "false"
        }
        flush stdout
        destroy .
        exit 0
    }

    proc on_settings_cancel {} {
        destroy .
        exit 1
    }

    wm title . [app_settings_window_title]
    wm resizable . 0 0

    ttk::label .download_lbl -text "Default folder:"
    ttk::entry .download_entry -width 38
    ttk::button .download_browse_btn -text "Browse..." -command on_settings_browse
    grid .download_lbl -row 0 -column 0 -columnspan 3 -sticky w -padx 10 -pady {10 2}
    grid .download_entry -row 1 -column 0 -columnspan 2 -sticky ew -padx {10 0}
    grid .download_browse_btn -row 1 -column 2 -sticky e -padx {4 10}
    .download_entry insert 0 $current_download_dir

    ttk::label .ytdlp_lbl -text "yt-dlp:"
    ttk::combobox .ytdlp_combo -state readonly -width 16
    ttk::label .ffmpeg_lbl -text "ffmpeg:"
    ttk::combobox .ffmpeg_combo -state readonly -width 16
    grid .ytdlp_lbl -row 2 -column 0 -sticky w -padx 10 -pady {10 2}
    grid .ytdlp_combo -row 3 -column 0 -sticky w -padx 10
    grid .ffmpeg_lbl -row 2 -column 1 -sticky w -padx 10 -pady {10 2}
    grid .ffmpeg_combo -row 3 -column 1 -sticky w -padx 10
    set_combo_value .ytdlp_combo $current_ytdlp {auto path bundled}
    set_combo_value .ffmpeg_combo $current_ffmpeg {auto path bundled}

    ttk::label .mode_lbl -text "GUI download:"
    ttk::combobox .mode_combo -state readonly -width 16
    set ::logs_var [bool_value $current_logs]
    ttk::checkbutton .logs_check -text "Create download logs" -variable ::logs_var
    grid .mode_lbl -row 4 -column 0 -sticky w -padx 10 -pady {10 2}
    grid .mode_combo -row 5 -column 0 -sticky w -padx 10
    grid .logs_check -row 5 -column 1 -columnspan 2 -sticky w -padx 10
    set_combo_value .mode_combo $current_gui_mode {progress external_cli}

    ttk::button .save_btn -text "Save" -command on_settings_save -default active
    ttk::button .settings_cancel_btn -text "Cancel" -command on_settings_cancel
    grid .save_btn -row 6 -column 1 -sticky e -padx 5 -pady 12
    grid .settings_cancel_btn -row 6 -column 2 -sticky e -padx {0 10} -pady 12

    grid columnconfigure . 0 -weight 1
    grid columnconfigure . 1 -weight 1

    wm protocol . WM_DELETE_WINDOW on_settings_cancel
    bind . <Return> on_settings_save
    bind . <Escape> on_settings_cancel
} elseif {[llength $argv] > 0 && [lindex $argv 0] eq "--progress"} {
    set logs_dir ""
    if {[llength $argv] > 1} {
        set logs_dir [lindex $argv 1]
    }

    set ::download_finished 0

    proc truncate_status {text {max 72}} {
        if {[string length $text] > $max} {
            return "[string range $text 0 [expr {$max - 3}]]..."
        }
        return $text
    }

    wm title . [app_window_title]
    wm resizable . 0 0
    wm geometry . 400x110

    ttk::label .status_lbl -text "Starting download..." -wraplength 376 -justify left
    ttk::progressbar .bar -mode determinate -maximum 100 -length 376
    grid .status_lbl -row 0 -column 0 -sticky ew -padx 12 -pady {12 6}
    grid .bar -row 1 -column 0 -sticky ew -padx 12 -pady {0 12}
    grid columnconfigure . 0 -minsize 376 -weight 0

    proc finish_download {exit_code logs_dir} {
        if {$::download_finished} {
            return
        }
        set ::download_finished 1
        destroy .
        exit $exit_code
    }

    proc apply_progress_line {line logs_dir} {
        set parts [split $line "\t"]
        set kind [lindex $parts 0]
        set payload [join [lrange $parts 1 end] "\t"]
        if {$kind eq "PROGRESS"} {
            if {[string is double -strict $payload]} {
                .bar configure -value $payload
            }
        } elseif {$kind eq "STATUS"} {
            .status_lbl configure -text [truncate_status $payload]
        } elseif {$kind eq "DONE"} {
            if {![string is integer -strict $payload]} {
                set payload 1
            }
            finish_download $payload $logs_dir
        }
    }

    proc on_stdin {logs_dir} {
        if {$::download_finished} {
            return
        }
        if {[gets stdin line] < 0} {
            if {[eof stdin]} {
                finish_download 1 $logs_dir
            }
            return
        }
        apply_progress_line $line $logs_dir
    }

    fconfigure stdin -blocking 0 -buffering line
    fileevent stdin readable [list on_stdin $logs_dir]

    wm protocol . WM_DELETE_WINDOW { finish_download 1 $logs_dir }
    bind . <Escape> { finish_download 1 $logs_dir }
} else {
    # Main download form (default when argv is output-dir path only)

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

proc on_settings {} {
    puts "__OPEN_SETTINGS__"
    flush stdout
    destroy .
    exit 0
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

wm title . [app_window_title]
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

ttk::button .settings_btn -text "\u2699" -width 3 -command on_settings
ttk::button .dl_btn -text "Download" -command on_download -default active
ttk::button .cancel_btn -text "Cancel" -command on_cancel
grid .settings_btn -row 7 -column 0 -sticky w -padx 10 -pady 12
grid .dl_btn -row 7 -column 1 -sticky e -padx 5 -pady 12
grid .cancel_btn -row 7 -column 2 -sticky e -padx {0 10} -pady 12

grid columnconfigure . 0 -weight 1
grid columnconfigure . 1 -weight 0

wm protocol . WM_DELETE_WINDOW on_cancel
set_formats

bind . <Return> on_download
bind . <Escape> on_cancel

}
