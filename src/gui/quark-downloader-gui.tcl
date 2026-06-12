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
set SPACES_VALUES {keep underscore dash remove}
set ::update_check_running 0
set ::current_gui_theme light

proc normalize_theme {value} {
    set lowered [string tolower $value]
    if {$lowered eq "dark"} {
        return dark
    }
    return light
}

proc style_configure {style args} {
    catch {ttk::style configure $style {*}$args}
}

proc style_map {style args} {
    catch {ttk::style map $style {*}$args}
}

proc apply_gui_theme {theme} {
    set ::current_gui_theme [normalize_theme $theme]

    catch {ttk::style theme use clam}

    if {$::current_gui_theme eq "dark"} {
        set bg "#1f2328"
        set surface "#2b3037"
        set active "#3a424d"
        set field "#15191f"
        set fg "#f0f3f6"
        set muted "#8b949e"
        set accent "#3d8bfd"
        set select_fg "#ffffff"
    } else {
        set bg "#f5f6f8"
        set surface "#ffffff"
        set active "#e8edf3"
        set field "#ffffff"
        set fg "#1f2328"
        set muted "#6b7280"
        set accent "#2563eb"
        set select_fg "#ffffff"
    }

    catch {tk_setPalette background $bg foreground $fg activeBackground $active activeForeground $fg selectBackground $accent selectForeground $select_fg}
    option add *Background $bg
    option add *Foreground $fg
    option add *selectBackground $accent
    option add *selectForeground $select_fg

    style_configure . -background $bg -foreground $fg -fieldbackground $field -selectbackground $accent -selectforeground $select_fg
    style_configure TFrame -background $bg
    style_configure TLabel -background $bg -foreground $fg
    style_configure TLabelframe -background $bg -bordercolor $active
    style_configure TLabelframe.Label -background $bg -foreground $muted
    style_configure TButton -background $surface -foreground $fg -focuscolor $accent
    style_configure TCheckbutton -background $bg -foreground $fg -focuscolor $accent
    style_configure TRadiobutton -background $bg -foreground $fg -focuscolor $accent
    style_configure TEntry -fieldbackground $field -foreground $fg -insertcolor $fg
    style_configure TCombobox -fieldbackground $field -background $surface -foreground $fg -arrowcolor $fg
    style_configure Horizontal.TProgressbar -background $accent -troughcolor $surface
    style_map TButton -background [list active $active disabled $surface] -foreground [list disabled $muted]
    style_map TCheckbutton -background [list active $bg] -foreground [list disabled $muted]
    style_map TRadiobutton -background [list active $bg] -foreground [list disabled $muted]
    style_map TEntry -fieldbackground [list readonly $field disabled $surface] -foreground [list disabled $muted]
    style_map TCombobox -fieldbackground [list readonly $field disabled $surface] -foreground [list disabled $muted] -selectbackground [list readonly $accent] -selectforeground [list readonly $select_fg]

    if {[winfo exists .main.queue_list]} {
        .main.queue_list configure -background $field -foreground $fg \
            -selectbackground $accent -selectforeground $select_fg \
            -highlightthickness 0 -borderwidth 1
    }
}

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

proc invoke_check_for_updates {} {
    if {$::update_check_running} {
        return
    }

    set script_dir [file dirname [info script]]
    set gui [file join $script_dir quark-downloader-gui]
    if {$::tcl_platform(platform) eq "windows"} {
        set gui "${gui}.exe"
    }
    if {![file exists $gui]} {
        tk_messageBox -title $::APP_NAME \
            -message "quark-downloader-gui was not found next to the GUI script." \
            -type ok -icon error
        return
    }

    set ::update_check_running 1
    if {[winfo exists .settings.updates_btn]} {
        .settings.updates_btn configure -text "Checking..." -state disabled
    }
    update idletasks

    if {[catch {exec -- $gui --check-updates &} err]} {
        set ::update_check_running 0
        if {[winfo exists .settings.updates_btn]} {
            .settings.updates_btn configure -text "Check for updates..." -state normal
        }
        tk_messageBox -title $::APP_NAME \
            -message "Could not check for updates:\n$err" \
            -type ok -icon error
        return
    }

    after 1500 {
        set ::update_check_running 0
        if {[winfo exists .settings.updates_btn]} {
            .settings.updates_btn configure -text "Check for updates..." -state normal
        }
    }
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
    set current_theme light
    set current_strip_ids true
    set current_sanitize true
    set current_spaces keep
    set current_playlist_folders true

    if {[llength $argv] > 1} { set default_dir [lindex $argv 1] }
    if {[llength $argv] > 2} { set current_download_dir [lindex $argv 2] }
    if {[llength $argv] > 3} { set current_ytdlp [lindex $argv 3] }
    if {[llength $argv] > 4} { set current_ffmpeg [lindex $argv 4] }
    if {[llength $argv] > 5} { set current_gui_mode [lindex $argv 5] }
    if {[llength $argv] > 6} { set current_logs [lindex $argv 6] }
    if {[llength $argv] > 7} { set current_theme [lindex $argv 7] }
    if {[llength $argv] > 8} { set current_strip_ids [lindex $argv 8] }
    if {[llength $argv] > 9} { set current_sanitize [lindex $argv 9] }
    if {[llength $argv] > 10} { set current_spaces [lindex $argv 10] }
    if {[llength $argv] > 11} { set current_playlist_folders [lindex $argv 11] }

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
    set ::session_theme [normalize_theme $current_theme]
    set ::session_strip_ids [session_bool_value $current_strip_ids]
    set ::session_strip_ids_var $::session_strip_ids
    set ::session_sanitize [session_bool_value $current_sanitize]
    set ::session_sanitize_var $::session_sanitize
    set ::session_spaces $current_spaces
    set ::session_playlist_folders [session_bool_value $current_playlist_folders]
    set ::session_playlist_folders_var $::session_playlist_folders

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

        .settings.general.download_entry delete 0 end
        .settings.general.download_entry insert 0 $::session_download_dir
        session_set_combo_value .settings.general.theme_combo $::session_theme {light dark}
        set ::session_strip_ids_var $::session_strip_ids
        set ::session_sanitize_var $::session_sanitize
        session_set_combo_value .settings.naming.spaces_combo $::session_spaces $::SPACES_VALUES
        set ::session_playlist_folders_var $::session_playlist_folders
        session_set_combo_value .settings.downloads.mode_combo $::session_gui_mode {progress external_cli}
        set ::session_logs_var $::session_logs
        session_set_combo_value .settings.tools.ytdlp_combo $::session_ytdlp {auto path bundled}
        session_set_combo_value .settings.tools.ffmpeg_combo $::session_ffmpeg {auto path bundled}

        grid .settings -row 0 -column 0 -sticky nsew
        session_bind_settings_keys
        focus .settings.general.download_entry
    }

    proc session_emit_settings {} {
        puts "__SETTINGS__"
        puts $::session_download_dir
        puts $::session_ytdlp
        puts $::session_ffmpeg
        puts $::session_gui_mode
        puts [expr {$::session_logs ? "true" : "false"}]
        puts $::session_theme
        puts [expr {$::session_strip_ids ? "true" : "false"}]
        puts [expr {$::session_sanitize ? "true" : "false"}]
        puts $::session_spaces
        puts [expr {$::session_playlist_folders ? "true" : "false"}]
    }

    proc session_emit_download {urls media_type format output} {
        puts "__SESSION__"
        if {$::session_settings_saved} {
            session_emit_settings
        }
        puts "__DOWNLOAD_MULTI__"
        puts [llength $urls]
        foreach url $urls {
            puts $url
        }
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

    proc session_queue_urls {} {
        return [.main.queue_list get 0 end]
    }

    proc session_on_add {} {
        set url [string trim [.main.url_entry get]]
        if {$url eq ""} {
            return
        }
        if {[lsearch -exact [session_queue_urls] $url] >= 0} {
            .main.url_entry delete 0 end
            return
        }
        .main.queue_list insert end $url
        .main.url_entry delete 0 end
        focus .main.url_entry
    }

    proc session_on_remove {} {
        set selected [.main.queue_list curselection]
        foreach idx [lreverse $selected] {
            .main.queue_list delete $idx
        }
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
        set initial [string trim [.settings.general.download_entry get]]
        if {$initial eq ""} {
            set initial [file normalize "~/Downloads"]
        }
        set chosen [tk_chooseDirectory -parent . -mustexist 1 \
            -initialdir [session_normalize_dir $initial] \
            -title "Select default download folder"]
        if {$chosen ne ""} {
            .settings.general.download_entry delete 0 end
            .settings.general.download_entry insert 0 $chosen
        }
    }

    proc session_on_settings_save {} {
        set download_dir [string trim [.settings.general.download_entry get]]
        if {$download_dir eq ""} {
            tk_messageBox -parent . -title "Quark Downloader" \
                -message "Please choose a default download folder." -type ok -icon error
            return
        }

        set previous_default $::session_default_dir
        set normalized_download_dir [session_normalize_dir $download_dir]
        set current_output [string trim [.main.output_entry get]]

        set ::session_download_dir $download_dir
        set ::session_theme [normalize_theme [.settings.general.theme_combo get]]
        set ::session_strip_ids $::session_strip_ids_var
        set ::session_sanitize $::session_sanitize_var
        set ::session_spaces [.settings.naming.spaces_combo get]
        set ::session_playlist_folders $::session_playlist_folders_var
        set ::session_gui_mode [.settings.downloads.mode_combo get]
        set ::session_logs $::session_logs_var
        set ::session_ytdlp [.settings.tools.ytdlp_combo get]
        set ::session_ffmpeg [.settings.tools.ffmpeg_combo get]
        set ::session_default_dir $normalized_download_dir
        set ::session_settings_saved 1
        apply_gui_theme $::session_theme

        if {$current_output eq "" || $current_output eq $previous_default} {
            .main.output_entry delete 0 end
            .main.output_entry insert 0 $normalized_download_dir
        }

        session_show_main
    }

    proc session_on_download {} {
        session_on_add

        set urls [session_queue_urls]
        if {[llength $urls] == 0} {
            tk_messageBox -parent . -title "Quark Downloader" \
                -message "Please enter at least one video or playlist URL." -type ok -icon error
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

        session_emit_download $urls $::session_media_type $format $output
    }

    wm title . [app_window_title]
    wm resizable . 0 0

    ttk::frame .main
    ttk::frame .settings
    grid columnconfigure . 0 -weight 1
    grid rowconfigure . 0 -weight 1

    # --- Main view ---------------------------------------------------------
    ttk::label .main.url_lbl -text "Video or playlist URL:"
    ttk::entry .main.url_entry -width 44
    ttk::button .main.add_btn -text "Add" -width 6 -command session_on_add
    grid .main.url_lbl -row 0 -column 0 -columnspan 3 -sticky w -padx 10 -pady {10 2}
    grid .main.url_entry -row 1 -column 0 -columnspan 2 -sticky ew -padx {10 0}
    grid .main.add_btn -row 1 -column 2 -sticky e -padx {4 10}

    ttk::label .main.queue_lbl -text "Queue:"
    listbox .main.queue_list -height 4 -activestyle none
    ttk::button .main.remove_btn -text "Remove" -width 8 -command session_on_remove
    grid .main.queue_lbl -row 2 -column 0 -sticky w -padx 10 -pady {8 2}
    grid .main.remove_btn -row 2 -column 2 -sticky e -padx {4 10} -pady {8 2}
    grid .main.queue_list -row 3 -column 0 -columnspan 3 -sticky ew -padx 10 -pady {0 8}

    ttk::radiobutton .main.rb_video -text "Video" -variable ::session_type_var -value video \
        -command session_on_type_change
    ttk::radiobutton .main.rb_audio -text "Audio" -variable ::session_type_var -value audio \
        -command session_on_type_change
    ttk::label .main.fmt_lbl -text "Format:"
    ttk::combobox .main.format_combo -state readonly -width 12
    grid .main.rb_video -row 4 -column 0 -sticky w -padx {10 0}
    grid .main.rb_audio -row 4 -column 1 -sticky w -padx 5
    grid .main.fmt_lbl -row 5 -column 0 -sticky w -padx 10 -pady {8 2}
    grid .main.format_combo -row 6 -column 0 -columnspan 2 -sticky w -padx 10 -pady {0 8}

    ttk::label .main.out_lbl -text "Output folder:"
    ttk::entry .main.output_entry -width 34
    ttk::button .main.browse_btn -text "Browse..." -command session_on_browse
    grid .main.out_lbl -row 7 -column 0 -columnspan 3 -sticky w -padx 10 -pady {0 2}
    grid .main.output_entry -row 8 -column 0 -columnspan 2 -sticky ew -padx {10 0}
    grid .main.browse_btn -row 8 -column 2 -sticky e -padx {4 10}

    .main.output_entry insert 0 $::session_default_dir

    ttk::button .main.settings_btn -text "⚙" -width 3 -command session_show_settings
    ttk::button .main.dl_btn -text "Download" -command session_on_download -default active
    ttk::button .main.cancel_btn -text "Close" -command session_emit_cancel
    grid .main.settings_btn -row 9 -column 0 -sticky w -padx 10 -pady 12
    grid .main.dl_btn -row 9 -column 1 -sticky e -padx 5 -pady 12
    grid .main.cancel_btn -row 9 -column 2 -sticky e -padx {0 10} -pady 12

    grid columnconfigure .main 0 -weight 1
    grid columnconfigure .main 1 -weight 0

    # --- Settings view -----------------------------------------------------
    ttk::labelframe .settings.general -text "General"
    ttk::label .settings.general.download_lbl -text "Default download folder:"
    ttk::entry .settings.general.download_entry -width 34
    ttk::button .settings.general.download_browse_btn -text "Browse..." \
        -command session_on_settings_browse
    ttk::label .settings.general.theme_lbl -text "Theme:"
    ttk::combobox .settings.general.theme_combo -state readonly -width 12
    grid .settings.general.download_lbl -row 0 -column 0 -columnspan 3 -sticky w -padx 8 -pady {6 2}
    grid .settings.general.download_entry -row 1 -column 0 -columnspan 2 -sticky ew -padx {8 0}
    grid .settings.general.download_browse_btn -row 1 -column 2 -sticky e -padx {4 8}
    grid .settings.general.theme_lbl -row 2 -column 0 -sticky w -padx 8 -pady {8 6}
    grid .settings.general.theme_combo -row 2 -column 1 -sticky w -padx 4 -pady {8 6}
    grid columnconfigure .settings.general 0 -weight 1

    ttk::labelframe .settings.naming -text "Download Naming"
    ttk::checkbutton .settings.naming.strip_check -text "Remove trailing video ID from filenames" \
        -variable ::session_strip_ids_var
    ttk::checkbutton .settings.naming.sanitize_check -text "Sanitize filenames (ASCII-safe)" \
        -variable ::session_sanitize_var
    ttk::label .settings.naming.spaces_lbl -text "Spaces in filenames:"
    ttk::combobox .settings.naming.spaces_combo -state readonly -width 12
    ttk::checkbutton .settings.naming.playlist_check -text "Put playlists in their own folder" \
        -variable ::session_playlist_folders_var
    grid .settings.naming.strip_check -row 0 -column 0 -columnspan 2 -sticky w -padx 8 -pady {6 2}
    grid .settings.naming.sanitize_check -row 1 -column 0 -columnspan 2 -sticky w -padx 8 -pady 2
    grid .settings.naming.spaces_lbl -row 2 -column 0 -sticky w -padx 8 -pady 2
    grid .settings.naming.spaces_combo -row 2 -column 1 -sticky w -padx 4 -pady 2
    grid .settings.naming.playlist_check -row 3 -column 0 -columnspan 2 -sticky w -padx 8 -pady {2 6}
    grid columnconfigure .settings.naming 0 -weight 1

    ttk::labelframe .settings.downloads -text "Downloads"
    ttk::label .settings.downloads.mode_lbl -text "Download window:"
    ttk::combobox .settings.downloads.mode_combo -state readonly -width 12
    ttk::checkbutton .settings.downloads.logs_check -text "Create download logs" \
        -variable ::session_logs_var
    grid .settings.downloads.mode_lbl -row 0 -column 0 -sticky w -padx 8 -pady {6 2}
    grid .settings.downloads.mode_combo -row 0 -column 1 -sticky w -padx 4 -pady {6 2}
    grid .settings.downloads.logs_check -row 1 -column 0 -columnspan 2 -sticky w -padx 8 -pady {2 6}
    grid columnconfigure .settings.downloads 0 -weight 1

    ttk::labelframe .settings.tools -text "Tools"
    ttk::label .settings.tools.ytdlp_lbl -text "yt-dlp:"
    ttk::combobox .settings.tools.ytdlp_combo -state readonly -width 12
    ttk::label .settings.tools.ffmpeg_lbl -text "ffmpeg:"
    ttk::combobox .settings.tools.ffmpeg_combo -state readonly -width 12
    grid .settings.tools.ytdlp_lbl -row 0 -column 0 -sticky w -padx 8 -pady {6 6}
    grid .settings.tools.ytdlp_combo -row 0 -column 1 -sticky w -padx 4 -pady {6 6}
    grid .settings.tools.ffmpeg_lbl -row 0 -column 2 -sticky w -padx 8 -pady {6 6}
    grid .settings.tools.ffmpeg_combo -row 0 -column 3 -sticky w -padx {4 8} -pady {6 6}
    grid columnconfigure .settings.tools 0 -weight 0

    grid .settings.general -row 0 -column 0 -columnspan 3 -sticky ew -padx 10 -pady {10 4}
    grid .settings.naming -row 1 -column 0 -columnspan 3 -sticky ew -padx 10 -pady 4
    grid .settings.downloads -row 2 -column 0 -columnspan 3 -sticky ew -padx 10 -pady 4
    grid .settings.tools -row 3 -column 0 -columnspan 3 -sticky ew -padx 10 -pady 4

    ttk::button .settings.updates_btn -text "Check for updates..." \
        -command invoke_check_for_updates
    ttk::button .settings.save_btn -text "Save" -command session_on_settings_save -default active
    ttk::button .settings.cancel_btn -text "Cancel" -command session_show_main
    grid .settings.updates_btn -row 4 -column 0 -sticky w -padx 10 -pady 12
    grid .settings.save_btn -row 4 -column 1 -sticky e -padx 5 -pady 12
    grid .settings.cancel_btn -row 4 -column 2 -sticky e -padx {0 10} -pady 12

    grid columnconfigure .settings 0 -weight 1

    apply_gui_theme $::session_theme
    wm protocol . WM_DELETE_WINDOW session_emit_cancel
    session_set_formats
    session_show_main
} elseif {[llength $argv] > 0 && [lindex $argv 0] eq "--progress"} {
    set logs_dir ""
    set current_theme light
    if {[llength $argv] > 1} {
        set first_progress_arg [lindex $argv 1]
        if {[normalize_theme $first_progress_arg] eq [string tolower $first_progress_arg]} {
            set current_theme $first_progress_arg
        } else {
            set logs_dir $first_progress_arg
        }
    }
    if {[llength $argv] > 2} {
        set current_theme [lindex $argv 2]
    }

    set ::download_finished 0
    set ::progress_eta ""
    set ::progress_eta_last_update 0
    set ::progress_eta_update_after ""
    set ::ETA_UPDATE_MS 1500

    proc truncate_status {text {max 72}} {
        if {[string length $text] > $max} {
            return "[string range $text 0 [expr {$max - 3}]]..."
        }
        return $text
    }

    proc eta_text {} {
        if {$::progress_eta eq ""} {
            return "Time left: estimating..."
        }
        return "Time left: $::progress_eta left"
    }

    proc update_progress_title {} {
        if {$::progress_eta eq ""} {
            wm title . "[app_window_title] - estimating..."
        } else {
            wm title . "[app_window_title] - $::progress_eta left"
        }
    }

    proc apply_eta_display_update {} {
        if {![winfo exists .eta_lbl]} {
            return
        }
        .eta_lbl configure -text [eta_text]
        update_progress_title
        set ::progress_eta_last_update [clock milliseconds]
        set ::progress_eta_update_after ""
    }

    proc schedule_eta_display_update {} {
        set now [clock milliseconds]
        if {$::progress_eta_last_update == 0} {
            apply_eta_display_update
            return
        }

        set elapsed [expr {$now - $::progress_eta_last_update}]
        if {$elapsed >= $::ETA_UPDATE_MS} {
            if {$::progress_eta_update_after ne ""} {
                after cancel $::progress_eta_update_after
                set ::progress_eta_update_after ""
            }
            apply_eta_display_update
        } elseif {$::progress_eta_update_after eq ""} {
            set delay [expr {$::ETA_UPDATE_MS - $elapsed}]
            set ::progress_eta_update_after [after $delay apply_eta_display_update]
        }
    }

    proc cancel_eta_display_update {} {
        if {$::progress_eta_update_after ne ""} {
            after cancel $::progress_eta_update_after
            set ::progress_eta_update_after ""
        }
    }

    update_progress_title
    wm resizable . 0 0
    wm geometry . 400x150
    apply_gui_theme $current_theme

    ttk::label .status_lbl -text "Starting download..." -wraplength 376 -justify left
    ttk::label .queue_lbl -text "" -justify left
    ttk::progressbar .bar -mode determinate -maximum 100 -length 376
    ttk::label .eta_lbl -text [eta_text] -justify left
    grid .status_lbl -row 0 -column 0 -sticky ew -padx 12 -pady {12 2}
    grid .queue_lbl -row 1 -column 0 -sticky w -padx 12 -pady {0 6}
    grid .bar -row 2 -column 0 -sticky ew -padx 12 -pady {0 12}
    grid .eta_lbl -row 3 -column 0 -sticky w -padx 12 -pady {0 8}
    grid columnconfigure . 0 -minsize 376 -weight 0

    proc finish_download {exit_code logs_dir} {
        if {$::download_finished} {
            return
        }
        set ::download_finished 1
        cancel_eta_display_update
        destroy .
        exit $exit_code
    }

    proc ignore_progress_close {} {
    }

    proc apply_progress_line {line logs_dir} {
        set parts [split $line "\t"]
        set kind [lindex $parts 0]
        set payload [join [lrange $parts 1 end] "\t"]
        if {$kind eq "PROGRESS"} {
            if {[string is double -strict $payload]} {
                .bar configure -value $payload
            }
        } elseif {$kind eq "ETA"} {
            if {$payload ne ""} {
                set ::progress_eta $payload
                schedule_eta_display_update
            }
        } elseif {$kind eq "STATUS"} {
            .status_lbl configure -text [truncate_status $payload]
        } elseif {$kind eq "QUEUE"} {
            .queue_lbl configure -text [truncate_status $payload]
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

    wm protocol . WM_DELETE_WINDOW ignore_progress_close
    bind . <Escape> { finish_download 1 $logs_dir }
} else {
    puts stderr "usage: $argv0 --session ... | --progress | --message ..."
    exit 2
}
