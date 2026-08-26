pub fn run_launcher() {
    #[cfg(not(windows))]
    {
        eprintln!("[Launcher Error] Windows GUI is required");
    }

    #[cfg(windows)]
    if let Err(error) = unsafe { windows_ui::run_window() } {
        eprintln!("[Launcher Error] {error}");
    }
}

#[cfg(windows)]
mod windows_ui {
    use std::mem::size_of;

    use windows::core::{w, HSTRING};
    use windows::Win32::Foundation::{
        BOOL, COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
    };
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateEllipticRgn,
        CreateFontW, CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteDC, DeleteObject,
        DrawTextW, EndPaint, ExcludeClipRect, FillRect, FillRgn, GetDC, GetMonitorInfoW,
        GetTextExtentPoint32W, InvalidateRect, MonitorFromWindow, Polyline, PtInRect, RedrawWindow,
        ReleaseDC, RestoreDC, SaveDC, ScreenToClient, SelectObject, SetBkColor, SetBkMode,
        SetTextColor, UpdateWindow,
        CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, RDW_ALLCHILDREN, RDW_INVALIDATE, RDW_NOERASE, RDW_UPDATENOW,
        DEFAULT_CHARSET, DEFAULT_QUALITY, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT,
        DT_SINGLELINE,
        DT_VCENTER, FW_NORMAL, FW_SEMIBOLD, HBITMAP, HBRUSH, HDC, HGDIOBJ, MONITORINFO,
        MONITOR_DEFAULTTONEAREST, OUT_DEFAULT_PRECIS, PS_SOLID, SRCCOPY, TRANSPARENT,
        VARIABLE_PITCH,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Controls::{
        SetWindowTheme, DRAWITEMSTRUCT, EM_SETLIMITTEXT, EM_SETSEL,
        EM_SETMARGINS, MEASUREITEMSTRUCT,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, CreateWindowExW, DefWindowProcW, DispatchMessageW, EnumChildWindows,
        GetClientRect, GetCursorPos, GetMessageW, GetParent, GetSystemMetrics, GetWindowLongPtrW,
        GetWindowRect, GetWindowTextW, IsWindowVisible, IsZoomed, KillTimer, LoadCursorW,
        MessageBoxW,
        PostQuitMessage, RegisterClassW, SendMessageW, SetTimer, SetWindowLongPtrW, SetWindowPos,
        SetWindowTextW, ShowWindow, TranslateMessage, BN_CLICKED, BS_OWNERDRAW, CBS_DROPDOWNLIST,
        CBS_HASSTRINGS, GWLP_WNDPROC, UISF_HIDEFOCUS, UIS_SET, WM_CHANGEUISTATE, WM_UPDATEUISTATE,
        CBS_OWNERDRAWFIXED, CB_ADDSTRING, CB_GETCOUNT, CB_GETCURSEL, CB_GETLBTEXT, CB_RESETCONTENT,
        CB_SETCURSEL,
        CB_SETITEMHEIGHT, CBN_KILLFOCUS, CBN_SELCHANGE, CBN_SETFOCUS, CS_DBLCLKS, CS_HREDRAW,
        CS_VREDRAW, EN_CHANGE, EN_KILLFOCUS, EN_SETFOCUS, ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_MULTILINE,
        ES_NUMBER, ES_PASSWORD, ES_WANTRETURN, GWLP_USERDATA, HMENU, HTBOTTOM, HTBOTTOMLEFT,
        HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTCLOSE, HTLEFT, HTMAXBUTTON, HTMINBUTTON, HTRIGHT,
        HTTOP, HTTOPLEFT, HTTOPRIGHT, IDC_ARROW, MB_OK, MINMAXINFO, MSG, SC_CLOSE, SC_MAXIMIZE,
        SC_MINIMIZE, SC_RESTORE, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE,
        SW_SHOW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND,
        WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORBTN, WM_CTLCOLORSTATIC, WM_DESTROY,
        WM_DRAWITEM, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MEASUREITEM,
        WM_MOUSEMOVE, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDBLCLK,
        WM_NCLBUTTONDOWN, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_NCPAINT, WM_PAINT, WM_PRINTCLIENT,
        WM_SETCURSOR, WM_SETFONT, WM_SIZE, WM_SYSCOMMAND, WM_TIMER, WNDCLASSW, WS_BORDER,
        WS_CHILD, WS_CLIPCHILDREN, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE, WS_EX_STATICEDGE,
        WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE, WS_VSCROLL,
        GWL_EXSTYLE, GWL_STYLE,
    };

    use crate::launcher::theme::{
        snap_font_scale, to_colorref, Metrics, Palette, FONT_SCALE_DEFAULT, FONT_SCALES,
    };
    use crate::tts::{
        list_installed_voices, preferred_free_voice_id,
        sort_voices_for_selector, voice_selector_label, ElevenLabsVoice, InstalledVoice,
    };

    use super::super::{
        apply_start, apply_started, apply_stop, apply_stop_requested, list_elevenlabs_voices,
        play_elevenlabs_test_voice, play_test_voice_text, start_action_enabled, stop_action_enabled,
        apply_app_volume_when_available, get_app_volume_percent, set_app_volume_percent,
        test_llm_connection, validate_volume,
        CommentaryLanguage, CommentaryStyle, ConnectionProvider, GameType, LauncherConfig,
        LauncherStatus, PipelineSession, TtsProvider, UiLanguage, UiTheme,
        DEFAULT_CUSTOM_STYLE_PROMPT, MAX_CUSTOM_STYLE_PROMPT_CHARS, OPENROUTER_BASE_URL,
        SYSTEM_DEFAULT_VOICE,
    };

    const IDC_GAME: i32 = 101;
    const IDC_PROVIDER: i32 = 102;
    const IDC_BASE_URL: i32 = 103;
    const IDC_MODEL: i32 = 104;
    const IDC_API_KEY: i32 = 105;
    const IDC_VOICE: i32 = 106;
    const IDC_STYLE: i32 = 107;
    const IDC_VOLUME: i32 = 108;
    const IDC_PROMPT: i32 = 109;
    const IDC_TEST_CONN: i32 = 110;
    const IDC_TEST_VOICE: i32 = 111;
    const IDC_SAVE: i32 = 112;
    const IDC_RESET: i32 = 113;
    const IDC_START: i32 = 114;
    const IDC_STOP: i32 = 115;
    const IDC_NOTE: i32 = 116;
    const IDC_NAV_HOME: i32 = 119;
    const IDC_NAV_SETTINGS: i32 = 120;
    const IDC_NAV_GENERAL: i32 = 121;
    const IDC_NAV_AI: i32 = 122;
    const IDC_NAV_STYLE: i32 = 123;
    const IDC_NAV_VOICE: i32 = 124;
    const IDC_EDIT_SETTINGS: i32 = 125;
    const IDC_PROMPT_HELP: i32 = 129;
    const IDC_STYLE_CHIP: i32 = 140;
    const IDC_SCALE_CHIP: i32 = 150;
    const IDC_UI_LANG: i32 = 160;
    const IDC_COMMENTARY_LANG: i32 = 161;
    const IDC_THEME_DARK: i32 = 162;
    const IDC_THEME_LIGHT: i32 = 163;
    const IDC_TTS_ENGINE: i32 = 164;
    const IDC_EL_API_KEY: i32 = 165;
    const IDC_EL_VOICE: i32 = 166;
    const IDC_EL_MODEL: i32 = 167;
    const IDC_EL_VOICE_ID: i32 = 168;
    const IDT_STATUS: usize = 1;
    const EC_BOTH_MARGINS: usize = 0x0001 | 0x0002;
    const ODS_COMBOBOXEDIT: u32 = 0x1000;
    const BM_SETSTATE: u32 = 0x00F3;
    static BUTTON_ORIG: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
    static COMBO_ORIG: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
    static PROMPT_ORIG: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Page {
        Home,
        Settings,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum SettingsSection {
        General,
        Ai,
        Style,
        Voice,
    }

    struct Fonts {
        title: windows::Win32::Graphics::Gdi::HFONT,
        subtitle: windows::Win32::Graphics::Gdi::HFONT,
        heading: windows::Win32::Graphics::Gdi::HFONT,
        brand: windows::Win32::Graphics::Gdi::HFONT,
        label: windows::Win32::Graphics::Gdi::HFONT,
        input: windows::Win32::Graphics::Gdi::HFONT,
        button: windows::Win32::Graphics::Gdi::HFONT,
        status: windows::Win32::Graphics::Gdi::HFONT,
        small: windows::Win32::Graphics::Gdi::HFONT,
    }

    struct UiState {
        metrics: Metrics,
        palette: Palette,
        theme: UiTheme,
        bg: HBRUSH,
        sidebar: HBRUSH,
        caption: HBRUSH,
        surface: HBRUSH,
        elevated: HBRUSH,
        prompt_bg: HBRUSH,
        fonts: Fonts,
        page: Page,
        settings_section: SettingsSection,
        ui_language: UiLanguage,
        commentary_language: CommentaryLanguage,
        game: HWND,
        provider: HWND,
        base_url: HWND,
        model: HWND,
        api_key: HWND,
        voice: HWND,
        tts_engine: HWND,
        el_api_key: HWND,
        el_voice: HWND,
        el_voice_id: HWND,
        el_model: HWND,
        style: HWND,
        style_chips: [HWND; 5],
        scale_chips: [HWND; 4],
        ui_lang: HWND,
        commentary_lang: HWND,
        theme_dark: HWND,
        theme_light: HWND,
        volume: HWND,
        label_app_volume: HWND,
        prompt: HWND,
        prompt_label: HWND,
        prompt_help: HWND,
        reset: HWND,
        save: HWND,
        test_conn: HWND,
        test_voice: HWND,
        start: HWND,
        stop: HWND,
        note: HWND,
        nav_home: HWND,
        nav_settings: HWND,
        nav_general: HWND,
        nav_ai: HWND,
        nav_style: HWND,
        nav_voice: HWND,
        edit_settings: HWND,
        label_game: HWND,
        label_language: HWND,
        label_commentary: HWND,
        label_appearance: HWND,
        label_provider: HWND,
        label_base_url: HWND,
        label_model: HWND,
        label_api_key: HWND,
        label_voice: HWND,
        label_tts_engine: HWND,
        label_el_api_key: HWND,
        label_el_voice_id: HWND,
        label_el_model: HWND,
        label_volume: HWND,
        label_scale: HWND,
        voices: Vec<InstalledVoice>,
        el_voices: Vec<ElevenLabsVoice>,
        el_voices_loaded: bool,
        status_value: LauncherStatus,
        session: Option<PipelineSession>,
        note_text: Option<String>,
        focused: HWND,
        slider_rect: RECT,
        app_slider_rect: RECT,
        prompt_count_rect: RECT,
        slider_drag: bool,
        app_slider_drag: bool,
        app_volume: u16,
        app_volume_available: bool,
        app_volume_pending: bool,
        sidebar_w: i32,
        settings_nav_w: i32,
        caption_hot: i32,
        hover_hwnd: HWND,
        last_status_sig: String,
        status_pulse: bool,
        mem_dc: HDC,
        mem_bitmap: HBITMAP,
        mem_old: HGDIOBJ,
        mem_w: i32,
        mem_h: i32,
    }

    pub unsafe fn run_window() -> Result<(), String> {
        let instance = GetModuleHandleW(None).map_err(|error| error.to_string())?;
        let class_name = w!("LolAiCommentaryLauncher");
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: class_name,
            hbrBackground: HBRUSH::default(),
            ..Default::default()
        };
        RegisterClassW(&class);

        let metrics = Metrics::new(FONT_SCALE_DEFAULT);
        let (width, height) = metrics.window_size();
        let x = ((GetSystemMetrics(SM_CXSCREEN) - width) / 2).max(24);
        let y = ((GetSystemMetrics(SM_CYSCREEN) - height) / 2).max(16);
        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name,
            w!("LOL AI Commentary"),
            WS_POPUP
                | WS_THICKFRAME
                | WS_MINIMIZEBOX
                | WS_MAXIMIZEBOX
                | WS_SYSMENU
                | WS_CLIPCHILDREN,
            x,
            y,
            width,
            height,
            HWND::default(),
            HMENU::default(),
            instance,
            None,
        )
        .map_err(|error| error.to_string())?;

        create_controls(hwnd)?;
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetTimer(hwnd, IDT_STATUS, 400, None);

        let mut message = MSG::default();
        while GetMessageW(&mut message, HWND::default(), 0, 0).into() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        Ok(())
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_NCCALCSIZE => LRESULT(0),
            WM_NCPAINT => LRESULT(0),
            WM_NCACTIVATE => LRESULT(1),
            WM_NCHITTEST => {
                let hit = hit_test(hwnd, lparam);
                update_caption_hot(hwnd, hit);
                LRESULT(hit as isize)
            }
            WM_NCLBUTTONDOWN => {
                handle_caption_button(hwnd, wparam.0 as i32, lparam);
                LRESULT(0)
            }
            WM_NCLBUTTONDBLCLK => {
                if wparam.0 as i32 == HTCAPTION as i32 {
                    toggle_maximize(hwnd);
                    LRESULT(0)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
            WM_SYSCOMMAND => DefWindowProcW(hwnd, msg, wparam, lparam),
            WM_NCMOUSEMOVE => {
                update_caption_hot(hwnd, hit_test(hwnd, lparam));
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_NCMOUSELEAVE => {
                update_caption_hot(hwnd, HTCLIENT as i32);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                paint_window(hwnd);
                LRESULT(0)
            }
            WM_CTLCOLORBTN | WM_CTLCOLORSTATIC => color_static(hwnd, wparam),
            WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => color_input(hwnd, wparam, lparam),
            WM_DRAWITEM => {
                draw_item(hwnd, lparam);
                LRESULT(1)
            }
            WM_MEASUREITEM => {
                let measure = lparam.0 as *mut MEASUREITEMSTRUCT;
                if !measure.is_null() {
                    let height = ui_state(hwnd)
                        .map(|state| state.metrics.input_h - 4)
                        .unwrap_or(36);
                    (*measure).itemHeight = height as u32;
                }
                LRESULT(1)
            }
            WM_SIZE => {
                resize_back_buffer(hwnd);
                apply_visual_metrics(hwnd);
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                on_slider_mouse(hwnd, lparam, true);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                if ui_state(hwnd).is_some_and(|state| state.caption_hot != 0) {
                    update_caption_hot(hwnd, HTCLIENT as i32);
                }
                update_hover_from_cursor(hwnd);
                on_slider_mouse(hwnd, lparam, false);
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                let _ = ReleaseCapture();
                if let Some(state) = ui_state(hwnd) {
                    let persist_app = state.app_slider_drag && state.app_volume_available;
                    state.slider_drag = false;
                    state.app_slider_drag = false;
                    if persist_app {
                        if let Ok(config) = read_config(state) {
                            let _ = config.save_to_disk();
                        }
                    }
                }
                LRESULT(0)
            }
            WM_SETCURSOR => {
                update_hover_from_cursor(hwnd);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_GETMINMAXINFO => {
                apply_minmax(hwnd, lparam);
                LRESULT(0)
            }
            WM_COMMAND => {
                let id = (wparam.0 as u32) & 0xffff;
                let code = ((wparam.0 as u32) >> 16) & 0xffff;
                if code == CBN_SELCHANGE as u32 {
                    if id == IDC_PROVIDER as u32 {
                        on_provider_changed(hwnd);
                    }
                    if id == IDC_STYLE as u32 {
                        on_style_changed(hwnd);
                    }
                    if id == IDC_UI_LANG as u32 {
                        on_ui_language_combo(hwnd);
                    }
                    if id == IDC_COMMENTARY_LANG as u32 {
                        on_commentary_language_combo(hwnd);
                    }
                    if id == IDC_TTS_ENGINE as u32 {
                        on_tts_engine_changed(hwnd);
                    }
                    if id == IDC_EL_VOICE as u32 {
                        on_elevenlabs_voice_changed(hwnd);
                    }
                    let _ = InvalidateRect(hwnd, None, false);
                }
                if code == EN_CHANGE as u32 && id == IDC_PROMPT as u32 {
                    enforce_prompt_limit(hwnd);
                }
                if code == EN_SETFOCUS as u32 || code == CBN_SETFOCUS as u32 {
                    let control = HWND(lparam.0 as *mut core::ffi::c_void);
                    if let Some(state) = ui_state(hwnd) {
                        state.focused = control;
                    }
                    let _ = InvalidateRect(hwnd, None, false);
                    let _ = InvalidateRect(control, None, false);
                }
                if code == EN_KILLFOCUS as u32 || code == CBN_KILLFOCUS as u32 {
                    let control = HWND(lparam.0 as *mut core::ffi::c_void);
                    if let Some(state) = ui_state(hwnd) {
                        state.focused = HWND::default();
                    }
                    let _ = InvalidateRect(hwnd, None, false);
                    let _ = InvalidateRect(control, None, false);
                    if code == EN_KILLFOCUS as u32 && id == IDC_EL_API_KEY as u32 {
                        refresh_elevenlabs_voices(hwnd, true);
                    }
                }
                if code == BN_CLICKED as u32 {
                    match id as i32 {
                        IDC_START => on_start(hwnd),
                        IDC_STOP => on_stop(hwnd),
                        IDC_TEST_CONN => on_test_connection(hwnd),
                        IDC_TEST_VOICE => on_test_voice(hwnd),
                        IDC_SAVE => on_save(hwnd),
                        IDC_RESET => on_reset_prompt(hwnd),
                        IDC_NAV_HOME => set_page(hwnd, Page::Home, None),
                        IDC_NAV_SETTINGS => set_page(hwnd, Page::Settings, None),
                        IDC_EDIT_SETTINGS => {
                            set_page(hwnd, Page::Settings, Some(SettingsSection::Ai))
                        }
                        IDC_NAV_GENERAL => {
                            set_page(hwnd, Page::Settings, Some(SettingsSection::General))
                        }
                        IDC_NAV_AI => set_page(hwnd, Page::Settings, Some(SettingsSection::Ai)),
                        IDC_NAV_STYLE => {
                            set_page(hwnd, Page::Settings, Some(SettingsSection::Style))
                        }
                        IDC_NAV_VOICE => {
                            set_page(hwnd, Page::Settings, Some(SettingsSection::Voice))
                        }
                        IDC_THEME_DARK => set_ui_theme(hwnd, UiTheme::Dark),
                        IDC_THEME_LIGHT => set_ui_theme(hwnd, UiTheme::Light),
                        id if (IDC_STYLE_CHIP..IDC_STYLE_CHIP + 5).contains(&id) => {
                            on_style_chip(hwnd, (id - IDC_STYLE_CHIP) as usize)
                        }
                        id if (IDC_SCALE_CHIP..IDC_SCALE_CHIP + 4).contains(&id) => {
                            set_interface_scale(
                                hwnd,
                                FONT_SCALES[(id - IDC_SCALE_CHIP) as usize],
                            )
                        }
                        _ => {}
                    }
                }
                LRESULT(0)
            }
            WM_TIMER => {
                if wparam.0 == IDT_STATUS {
                    if let Some(state) = ui_state(hwnd) {
                        if matches!(state.status_value, LauncherStatus::Starting) {
                            state.status_pulse = !state.status_pulse;
                        }
                        if refresh_app_volume_state(state) {
                            let _ = InvalidateRect(hwnd, None, false);
                        }
                    }
                    refresh_status_text(hwnd);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ = KillTimer(hwnd, IDT_STATUS);
                if let Some(mut state) = take_state(hwnd) {
                    if let Some(mut session) = state.session.take() {
                        if let Err(error) = session.stop_on_close() {
                            eprintln!("[Launcher Error] {error}");
                        }
                    }
                    destroy_fonts(&state.fonts);
                    release_back_buffer(&mut state);
                    let _ = DeleteObject(state.bg);
                    let _ = DeleteObject(state.sidebar);
                    let _ = DeleteObject(state.caption);
                    let _ = DeleteObject(state.surface);
                    let _ = DeleteObject(state.elevated);
                    let _ = DeleteObject(state.prompt_bg);
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    fn packed_point(lparam: LPARAM) -> POINT {
        POINT {
            x: (lparam.0 as u32 & 0xffff) as i16 as i32,
            y: ((lparam.0 as u32 >> 16) & 0xffff) as i16 as i32,
        }
    }

    unsafe fn hit_test(hwnd: HWND, lparam: LPARAM) -> i32 {
        let mut pt = packed_point(lparam);
        let _ = ScreenToClient(hwnd, &mut pt);
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let zoomed = IsZoomed(hwnd).as_bool();
        let (title_h, btn_w, edge) = ui_state(hwnd)
            .map(|state| {
                (
                    state.metrics.title_bar_h,
                    state.metrics.caption_btn_w(),
                    state.metrics.resize_border(),
                )
            })
            .unwrap_or((42, 46, 8));
        let in_caption = pt.y >= 0 && pt.y < title_h;
        let in_buttons = in_caption && pt.x >= client.right - btn_w * 3;
        if in_buttons {
            if pt.x >= client.right - btn_w {
                return HTCLOSE as i32;
            }
            if pt.x >= client.right - btn_w * 2 {
                return HTMAXBUTTON as i32;
            }
            return HTMINBUTTON as i32;
        }
        if !zoomed {
            let left = pt.x < edge;
            let right = pt.x >= client.right - edge;
            let top = pt.y < edge;
            let bottom = pt.y >= client.bottom - edge;
            if top && left {
                return HTTOPLEFT as i32;
            }
            if top && right {
                return HTTOPRIGHT as i32;
            }
            if bottom && left {
                return HTBOTTOMLEFT as i32;
            }
            if bottom && right {
                return HTBOTTOMRIGHT as i32;
            }
            if left {
                return HTLEFT as i32;
            }
            if right {
                return HTRIGHT as i32;
            }
            if top {
                return HTTOP as i32;
            }
            if bottom {
                return HTBOTTOM as i32;
            }
        }
        if in_caption {
            return HTCAPTION as i32;
        }
        HTCLIENT as i32
    }

    unsafe fn handle_caption_button(hwnd: HWND, hit: i32, lparam: LPARAM) {
        if hit == HTMINBUTTON as i32 {
            SendMessageW(hwnd, WM_SYSCOMMAND, WPARAM(SC_MINIMIZE as usize), LPARAM(0));
            return;
        }
        if hit == HTMAXBUTTON as i32 {
            toggle_maximize(hwnd);
            return;
        }
        if hit == HTCLOSE as i32 {
            SendMessageW(hwnd, WM_SYSCOMMAND, WPARAM(SC_CLOSE as usize), LPARAM(0));
            return;
        }
        DefWindowProcW(hwnd, WM_NCLBUTTONDOWN, WPARAM(hit as usize), lparam);
    }

    unsafe fn toggle_maximize(hwnd: HWND) {
        if IsZoomed(hwnd).as_bool() {
            SendMessageW(hwnd, WM_SYSCOMMAND, WPARAM(SC_RESTORE as usize), LPARAM(0));
        } else {
            SendMessageW(hwnd, WM_SYSCOMMAND, WPARAM(SC_MAXIMIZE as usize), LPARAM(0));
        }
    }

    unsafe fn update_caption_hot(hwnd: HWND, hit: i32) {
        let hot = if hit == HTCLOSE as i32
            || hit == HTMINBUTTON as i32
            || hit == HTMAXBUTTON as i32
        {
            hit
        } else {
            0
        };
        let changed = ui_state(hwnd).is_some_and(|state| {
            if state.caption_hot != hot {
                state.caption_hot = hot;
                true
            } else {
                false
            }
        });
        if changed {
            invalidate_caption(hwnd);
        }
    }

    unsafe fn invalidate_caption(hwnd: HWND) {
        let height = ui_state(hwnd)
            .map(|state| state.metrics.title_bar_h)
            .unwrap_or(42);
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let rect = RECT {
            left: 0,
            top: 0,
            right: client.right,
            bottom: height,
        };
        let _ = InvalidateRect(hwnd, Some(&rect), false);
    }

    unsafe fn apply_minmax(hwnd: HWND, lparam: LPARAM) {
        let info = lparam.0 as *mut MINMAXINFO;
        if info.is_null() {
            return;
        }
        (*info).ptMinTrackSize.x = 820;
        (*info).ptMinTrackSize.y = 680;
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut mi).as_bool() {
            let work = mi.rcWork;
            let screen = mi.rcMonitor;
            (*info).ptMaxPosition.x = work.left - screen.left;
            (*info).ptMaxPosition.y = work.top - screen.top;
            (*info).ptMaxSize.x = work.right - work.left;
            (*info).ptMaxSize.y = work.bottom - work.top;
            (*info).ptMaxTrackSize.x = work.right - work.left;
            (*info).ptMaxTrackSize.y = work.bottom - work.top;
        }
    }

    unsafe fn paint_window(hwnd: HWND) {
        let mut paint = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut paint);
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let width = (client.right - client.left).max(1);
        let height = (client.bottom - client.top).max(1);
        if let Some(mem) = ensure_back_buffer(hwnd, hdc, width, height) {
            SetBkMode(mem, TRANSPARENT);
            paint_background(hwnd, mem);
            paint_chrome(hwnd, mem);
            paint_field_focus(hwnd, mem);
            let saved = SaveDC(hdc);
            exclude_opaque_children(hwnd, hdc);
            let _ = BitBlt(hdc, 0, 0, width, height, mem, 0, 0, SRCCOPY);
            let _ = RestoreDC(hdc, saved);
        }
        let _ = EndPaint(hwnd, &paint);
    }

    unsafe fn exclude_opaque_children(parent: HWND, hdc: HDC) {
        let Some(state) = ui_state(parent) else {
            return;
        };
        let mut children = owner_buttons(state);
        children.extend([
            state.game,
            state.provider,
            state.ui_lang,
            state.commentary_lang,
            state.base_url,
            state.model,
            state.api_key,
            state.voice,
            state.tts_engine,
            state.style,
            state.volume,
            state.prompt,
            state.el_api_key,
            state.el_voice,
            state.el_voice_id,
            state.el_model,
        ]);
        for child in children {
            exclude_child_rect(parent, hdc, child);
        }
    }

    unsafe fn exclude_child_rect(parent: HWND, hdc: HDC, child: HWND) {
        if child.is_invalid() || !IsWindowVisible(child).as_bool() {
            return;
        }
        let mut screen = RECT::default();
        let _ = GetWindowRect(child, &mut screen);
        let mut top_left = POINT {
            x: screen.left,
            y: screen.top,
        };
        let mut bottom_right = POINT {
            x: screen.right,
            y: screen.bottom,
        };
        let _ = ScreenToClient(parent, &mut top_left);
        let _ = ScreenToClient(parent, &mut bottom_right);
        ExcludeClipRect(hdc, top_left.x, top_left.y, bottom_right.x, bottom_right.y);
    }

    unsafe fn resize_back_buffer(hwnd: HWND) {
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let hdc = GetDC(hwnd);
        let _ = ensure_back_buffer(
            hwnd,
            hdc,
            (client.right - client.left).max(1),
            (client.bottom - client.top).max(1),
        );
        let _ = ReleaseDC(hwnd, hdc);
    }

    unsafe fn release_back_buffer(state: &mut UiState) {
        if !state.mem_dc.is_invalid() {
            if !state.mem_old.is_invalid() {
                SelectObject(state.mem_dc, state.mem_old);
            }
            if !state.mem_bitmap.is_invalid() {
                let _ = DeleteObject(state.mem_bitmap);
            }
            let _ = DeleteDC(state.mem_dc);
        }
        state.mem_dc = HDC::default();
        state.mem_bitmap = HBITMAP::default();
        state.mem_old = HGDIOBJ::default();
        state.mem_w = 0;
        state.mem_h = 0;
    }

    unsafe fn ensure_back_buffer(hwnd: HWND, hdc: HDC, width: i32, height: i32) -> Option<HDC> {
        let state = ui_state(hwnd)?;
        if !state.mem_dc.is_invalid() && state.mem_w == width && state.mem_h == height {
            return Some(state.mem_dc);
        }
        release_back_buffer(state);
        if width <= 0 || height <= 0 {
            return None;
        }
        let dc = CreateCompatibleDC(hdc);
        if dc.is_invalid() {
            return None;
        }
        let bitmap = CreateCompatibleBitmap(hdc, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(dc);
            return None;
        }
        let old = SelectObject(dc, bitmap);
        state.mem_dc = dc;
        state.mem_bitmap = bitmap;
        state.mem_old = old;
        state.mem_w = width;
        state.mem_h = height;
        Some(dc)
    }

    unsafe fn owner_buttons(state: &UiState) -> Vec<HWND> {
        let mut buttons = vec![
            state.start,
            state.stop,
            state.test_conn,
            state.test_voice,
            state.save,
            state.reset,
            state.nav_home,
            state.nav_settings,
            state.nav_general,
            state.nav_ai,
            state.nav_style,
            state.nav_voice,
            state.edit_settings,
            state.theme_dark,
            state.theme_light,
        ];
        buttons.extend(state.style_chips);
        buttons.extend(state.scale_chips);
        buttons
    }

    unsafe fn set_hover_button(parent: HWND, button: HWND) {
        let Some(state) = ui_state(parent) else {
            return;
        };
        if state.hover_hwnd.0 == button.0 {
            return;
        }
        let prev = state.hover_hwnd;
        state.hover_hwnd = button;
        if !prev.is_invalid() {
            let _ = InvalidateRect(prev, None, false);
        }
        if !button.is_invalid() {
            let _ = InvalidateRect(button, None, false);
        }
    }

    unsafe fn update_hover_from_cursor(hwnd: HWND) {
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let mut found = HWND::default();
        let mut targets = owner_buttons(state);
        targets.extend(form_fields(state));
        for button in targets {
            if !IsWindowVisible(button).as_bool() {
                continue;
            }
            let mut rect = RECT::default();
            let _ = GetWindowRect(button, &mut rect);
            if PtInRect(&rect, cursor).as_bool() {
                found = button;
                break;
            }
        }
        set_hover_button(hwnd, found);
    }

    unsafe extern "system" fn owner_button_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_ERASEBKGND {
            return LRESULT(1);
        }
        if msg == WM_UPDATEUISTATE {
            return LRESULT(0);
        }
        if msg == WM_MOUSEMOVE {
            if let Ok(parent) = GetParent(hwnd) {
                set_hover_button(parent, hwnd);
            }
        }
        let orig = BUTTON_ORIG.load(std::sync::atomic::Ordering::Relaxed);
        if orig == 0 {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        } else {
            CallWindowProcW(
                Some(std::mem::transmute(orig)),
                hwnd,
                msg,
                wparam,
                lparam,
            )
        }
    }

    unsafe fn subclass_owner_button(hwnd: HWND) {
        let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, owner_button_proc as *const () as isize);
        let _ = BUTTON_ORIG.compare_exchange(
            0,
            orig,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    unsafe extern "system" fn combo_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_ERASEBKGND || msg == WM_NCPAINT {
            return LRESULT(0);
        }
        if msg == WM_MOUSEMOVE {
            if let Ok(parent) = GetParent(hwnd) {
                set_hover_button(parent, hwnd);
            }
        }
        let orig = COMBO_ORIG.load(std::sync::atomic::Ordering::Relaxed);
        let result = if orig == 0 {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        } else {
            CallWindowProcW(
                Some(std::mem::transmute(orig)),
                hwnd,
                msg,
                wparam,
                lparam,
            )
        };
        if msg == WM_PAINT || msg == WM_PRINTCLIENT {
            paint_flat_select(hwnd);
        }
        result
    }

    unsafe fn subclass_combo(hwnd: HWND) {
        flatten_form_control(hwnd);
        let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, combo_proc as *const () as isize);
        let _ = COMBO_ORIG.compare_exchange(
            0,
            orig,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    unsafe fn flatten_form_control(hwnd: HWND) {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(WS_BORDER.0 as isize));
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            ex & !((WS_EX_CLIENTEDGE.0 | WS_EX_STATICEDGE.0) as isize),
        );
        let _ = SetWindowPos(
            hwnd,
            HWND::default(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }

    unsafe extern "system" fn prompt_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_ERASEBKGND {
            fill_prompt_client(hwnd, HDC(wparam.0 as *mut core::ffi::c_void));
            return LRESULT(1);
        }
        if msg == WM_NCPAINT {
            return LRESULT(0);
        }
        if msg == WM_PAINT {
            let hdc = GetDC(hwnd);
            fill_prompt_client(hwnd, hdc);
            let _ = ReleaseDC(hwnd, hdc);
        }
        let orig = PROMPT_ORIG.load(std::sync::atomic::Ordering::Relaxed);
        if orig == 0 {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        } else {
            CallWindowProcW(
                Some(std::mem::transmute(orig)),
                hwnd,
                msg,
                wparam,
                lparam,
            )
        }
    }

    unsafe fn fill_prompt_client(hwnd: HWND, hdc: HDC) {
        let Ok(parent) = GetParent(hwnd) else {
            return;
        };
        let Some(state) = ui_state(parent) else {
            return;
        };
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let _ = FillRect(hdc, &client, state.prompt_bg);
    }

    unsafe fn subclass_prompt(hwnd: HWND) {
        flatten_form_control(hwnd);
        let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, prompt_proc as *const () as isize);
        let _ = PROMPT_ORIG.compare_exchange(
            0,
            orig,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    fn form_fields(state: &UiState) -> Vec<HWND> {
        vec![
            state.game,
            state.provider,
            state.ui_lang,
            state.commentary_lang,
            state.base_url,
            state.model,
            state.api_key,
            state.voice,
            state.tts_engine,
            state.el_api_key,
            state.el_voice,
            state.el_voice_id,
            state.el_model,
        ]
    }

    fn field_fill(palette: Palette, focused: bool, hovered: bool) -> (u8, u8, u8) {
        if focused || hovered {
            palette.elevated
        } else {
            palette.surface
        }
    }

    fn field_active(state: &UiState, control: HWND) -> (bool, bool) {
        let focused = !state.focused.is_invalid() && state.focused.0 == control.0;
        let hovered = !state.hover_hwnd.is_invalid() && state.hover_hwnd.0 == control.0;
        (focused, hovered)
    }

    unsafe fn paint_flat_field(
        hdc: HDC,
        rect: RECT,
        palette: Palette,
        fonts: &Fonts,
        focused: bool,
        hovered: bool,
        arrow: bool,
    ) {
        fill_color(hdc, rect, field_fill(palette, focused, hovered));
        if !arrow {
            return;
        }
        SelectObject(hdc, fonts.small);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(to_colorref(palette.text_muted)));
        let mut arrow_rect = RECT {
            left: (rect.right - 28).max(rect.left),
            top: rect.top,
            right: rect.right - 6,
            bottom: rect.bottom,
        };
        let mut chevron = wide("▾");
        DrawTextW(
            hdc,
            &mut chevron,
            &mut arrow_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }

    unsafe fn paint_flat_select(combo: HWND) {
        let Ok(parent) = GetParent(combo) else {
            return;
        };
        let Some(state) = ui_state(parent) else {
            return;
        };
        let mut client = RECT::default();
        let _ = GetClientRect(combo, &mut client);
        let (focused, hovered) = field_active(state, combo);
        let hdc = GetDC(combo);
        paint_flat_field(
            hdc,
            client,
            pal(state),
            &state.fonts,
            focused,
            hovered,
            true,
        );
        SelectObject(hdc, state.fonts.input);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        let mut text_rect = client;
        text_rect.left += 12;
        text_rect.right -= 28;
        let mut wide_text = wide(&combo_shown_text(state, combo));
        DrawTextW(
            hdc,
            &mut wide_text,
            &mut text_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
        let _ = ReleaseDC(combo, hdc);
    }

    fn pal(state: &UiState) -> Palette {
        state.palette
    }

    unsafe fn paint_background(hwnd: HWND, hdc: HDC) {
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let _ = FillRect(hdc, &client, state.bg);
        let title = RECT {
            left: 0,
            top: 0,
            right: client.right,
            bottom: state.metrics.title_bar_h,
        };
        let _ = FillRect(hdc, &title, state.caption);
        let sidebar = RECT {
            left: 0,
            top: state.metrics.title_bar_h,
            right: state.sidebar_w,
            bottom: client.bottom,
        };
        let _ = FillRect(hdc, &sidebar, state.sidebar);
        fill_color(
            hdc,
            RECT {
                left: state.sidebar_w,
                top: state.metrics.title_bar_h,
                right: state.sidebar_w + 1,
                bottom: client.bottom,
            },
            pal(state).border,
        );
        if pal(state).paper_ornaments || pal(state).ink_ornaments {
            crate::launcher::oriental_background::paint_landscape(
                hdc,
                RECT {
                    left: state.sidebar_w + 1,
                    top: state.metrics.title_bar_h,
                    right: client.right,
                    bottom: client.bottom,
                },
                pal(state),
            );
        }
        if !IsZoomed(hwnd).as_bool() {
            fill_color(
                hdc,
                RECT {
                    left: 0,
                    top: 0,
                    right: client.right,
                    bottom: 1,
                },
                pal(state).border,
            );
            fill_color(
                hdc,
                RECT {
                    left: 0,
                    top: client.bottom - 1,
                    right: client.right,
                    bottom: client.bottom,
                },
                pal(state).border,
            );
            fill_color(
                hdc,
                RECT {
                    left: 0,
                    top: 0,
                    right: 1,
                    bottom: client.bottom,
                },
                pal(state).border,
            );
            fill_color(
                hdc,
                RECT {
                    left: client.right - 1,
                    top: 0,
                    right: client.right,
                    bottom: client.bottom,
                },
                pal(state).border,
            );
        }
    }

    unsafe fn paint_chrome(hwnd: HWND, hdc: HDC) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let m = state.metrics;
        SetBkMode(hdc, TRANSPARENT);
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        paint_title_bar(hdc, state, hwnd, &client);
        paint_sidebar(hdc, state, client.bottom);

        let content_x = state.sidebar_w + m.pad;
        match state.page {
            Page::Home => paint_home(hdc, state, content_x, client.right - content_x - m.pad),
            Page::Settings => paint_settings_chrome(hdc, state, content_x),
        }
        if state.page == Page::Settings && state.settings_section == SettingsSection::Voice {
            paint_volume_sliders(hdc, state);
        }
    }

    unsafe fn paint_title_bar(hdc: HDC, state: &UiState, hwnd: HWND, client: &RECT) {
        let m = state.metrics;
        let cy = (m.title_bar_h - 10) / 2;
        fill_ellipse(hdc, 16, cy, 26, cy + 10, pal(state).gold);

        SelectObject(hdc, state.fonts.small);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        draw_left_text(
            hdc,
            34,
            0,
            280,
            m.title_bar_h,
            "LOL AI Commentary",
        );

        let btn_w = m.caption_btn_w();
        let labels = if IsZoomed(hwnd).as_bool() {
            ["─", "❐", "✕"]
        } else {
            ["─", "□", "✕"]
        };
        let hits = [HTMINBUTTON as i32, HTMAXBUTTON as i32, HTCLOSE as i32];
        for (index, (label, hit)) in labels.iter().zip(hits).enumerate() {
            let left = client.right - btn_w * (3 - index as i32);
            let hot = state.caption_hot == hit;
            if hot && hit == HTCLOSE as i32 {
                fill_color(
                    hdc,
                    RECT {
                        left,
                        top: 0,
                        right: left + btn_w,
                        bottom: m.title_bar_h,
                    },
                    pal(state).close_hover,
                );
            } else if hot {
                fill_color(
                    hdc,
                    RECT {
                        left,
                        top: 0,
                        right: left + btn_w,
                        bottom: m.title_bar_h,
                    },
                    pal(state).elevated,
                );
            }
            SelectObject(hdc, state.fonts.small);
            let color = if hot && hit == HTCLOSE as i32 {
                pal(state).text
            } else {
                pal(state).text_muted
            };
            SetTextColor(hdc, COLORREF(to_colorref(color)));
            draw_center_text(hdc, left, 0, btn_w, m.title_bar_h, label);
        }
    }

    unsafe fn paint_sidebar(hdc: HDC, state: &UiState, client_bottom: i32) {
        let m = state.metrics;
        let s = state.ui_language.strings();
        let top = m.title_bar_h + 22;
        fill_ellipse(hdc, 24, top, 36, top + 12, pal(state).green);

        SetBkMode(hdc, TRANSPARENT);
        SelectObject(hdc, state.fonts.brand);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        let text_x = 24;
        let text_w = (state.sidebar_w - 40).max(160);
        draw_left_text(hdc, text_x, top + 20, text_w, m.brand + 4, s.brand_title);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        draw_left_text(
            hdc,
            text_x,
            top + 20 + m.brand + 2,
            text_w,
            m.brand + 4,
            "COMMENTARY",
        );
        SelectObject(hdc, state.fonts.small);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text_muted)));
        draw_left_text(
            hdc,
            text_x,
            top + 20 + m.brand * 2 + 6,
            text_w,
            m.small + 4,
            s.brand_sub,
        );

        let accent_top = top + 20 + m.brand * 2 + m.small + 18;
        fill_color(
            hdc,
            RECT {
                left: text_x,
                top: accent_top,
                right: text_x + 36,
                bottom: accent_top + 1,
            },
            pal(state).gold,
        );

        SelectObject(hdc, state.fonts.small);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text_muted)));
        draw_left_text(
            hdc,
            text_x,
            client_bottom - m.small - 22,
            text_w,
            m.small + 4,
            "v1.0",
        );
    }

    unsafe fn paint_home(hdc: HDC, state: &UiState, x: i32, width: i32) {
        let m = state.metrics;
        let s = state.ui_language.strings();
        paint_heading(
            hdc,
            &state.fonts,
            x,
            m.title_bar_h + 16,
            s.header_title,
            s.header_sub,
            m,
            width.max(420),
            pal(state),
        );

        let hero_y = m.page_header_h();
        let hero_h = (m.start_h + 88).max(128);
        let hero = RECT {
            left: x,
            top: hero_y,
            right: x + width.max(420),
            bottom: hero_y + hero_h,
        };
        paint_hero(hdc, state, hero);

        let (hero_status, ai, obs, style) = status_values(state);
        let _ = hero_status;
        let mods_y = hero.bottom + 14;
        let gap = 10;
        let mod_w = ((width.max(420) - gap * 2) / 3).max(140);
        let mod_h = 68;
        paint_module(hdc, state, x, mods_y, mod_w, mod_h, "AI", &ai);
        paint_module(
            hdc,
            state,
            x + mod_w + gap,
            mods_y,
            mod_w,
            mod_h,
            "OBS",
            &obs,
        );
        paint_module(
            hdc,
            state,
            x + (mod_w + gap) * 2,
            mods_y,
            mod_w,
            mod_h,
            s.style,
            &style,
        );

        let config_y = mods_y + mod_h + 18;
        SelectObject(hdc, state.fonts.heading);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).gold)));
        draw_left_text(hdc, x, config_y, width, m.heading_h(), s.current_config);
        let pair_w = (width / 2).max(200);
        let mut y = config_y + m.heading_h() + 10;
        paint_meta(hdc, state, x, y, s.api, &combo_text(state.provider));
        paint_meta(hdc, state, x + pair_w, y, s.model, &edit_text(state.model));
        y += m.small + 8 + m.status + 12;
        paint_meta(hdc, state, x, y, s.voice, &combo_text(state.voice));
        paint_meta(hdc, state, x + pair_w, y, s.style, &combo_text(state.style));
    }

    unsafe fn paint_hero(hdc: HDC, state: &UiState, rect: RECT) {
        let s = state.ui_language.strings();
        fill_round_rect(hdc, rect, 6, pal(state).hero, None);
        paint_ink_lines(hdc, &rect, pal(state));
        fill_ellipse(
            hdc,
            rect.left + 22,
            rect.top + 20,
            rect.left + 30,
            rect.top + 28,
            hero_indicator_color(state),
        );

        SelectObject(hdc, state.fonts.small);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text_muted)));
        draw_left_text(
            hdc,
            rect.left + 38,
            rect.top + 16,
            360,
            state.metrics.small + 6,
            s.commentary_system,
        );

        let (hero, _, _, _) = status_values(state);
        SelectObject(hdc, state.fonts.title);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        draw_left_text(
            hdc,
            rect.left + 22,
            rect.top + 44,
            (rect.right - rect.left - 40).max(200),
            state.metrics.title + 8,
            &hero,
        );

        let waiting = match &state.status_value {
            LauncherStatus::Running => state
                .session
                .as_ref()
                .and_then(|session| session.tts_hint().status_line())
                .unwrap_or(""),
            LauncherStatus::Starting => s.start_starting,
            LauncherStatus::Stopping => s.stop_stopping,
            LauncherStatus::Error(message) => {
                if message.is_empty() {
                    s.start_failed_hint
                } else {
                    message.as_str()
                }
            }
            _ => s.waiting,
        };
        if !waiting.is_empty() {
            SelectObject(hdc, state.fonts.subtitle);
            SetTextColor(hdc, COLORREF(to_colorref(pal(state).text_muted)));
            draw_left_text(
                hdc,
                rect.left + 22,
                rect.top + 44 + state.metrics.title + 10,
                (rect.right - rect.left - 40).max(240),
                state.metrics.subtitle + 6,
                waiting,
            );
        }
    }

    unsafe fn paint_ink_lines(hdc: HDC, rect: &RECT, palette: Palette) {
        let pen = CreatePen(PS_SOLID, 1, COLORREF(to_colorref(palette.ink_line)));
        let old_pen = SelectObject(hdc, pen);
        let base_x = rect.right - 220;
        let base_y = rect.bottom - 18;
        if base_x > rect.left + 280 {
            let a = [
                POINT {
                    x: base_x,
                    y: base_y,
                },
                POINT {
                    x: base_x + 46,
                    y: rect.top + 38,
                },
                POINT {
                    x: base_x + 88,
                    y: base_y - 8,
                },
                POINT {
                    x: base_x + 128,
                    y: rect.top + 52,
                },
                POINT {
                    x: base_x + 186,
                    y: base_y,
                },
            ];
            let _ = Polyline(hdc, &a);
            let b = [
                POINT {
                    x: base_x + 24,
                    y: base_y,
                },
                POINT {
                    x: base_x + 70,
                    y: rect.top + 70,
                },
                POINT {
                    x: base_x + 150,
                    y: base_y,
                },
            ];
            let _ = Polyline(hdc, &b);
        }
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen);
    }

    unsafe fn paint_module(
        hdc: HDC,
        state: &UiState,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: &str,
        value: &str,
    ) {
        fill_round_rect(
            hdc,
            RECT {
                left: x,
                top: y,
                right: x + w,
                bottom: y + h,
            },
            5,
            pal(state).surface,
            None,
        );
        SelectObject(hdc, state.fonts.small);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text_muted)));
        draw_left_text(hdc, x + 14, y + 10, w - 24, state.metrics.small + 4, label);
        SelectObject(hdc, state.fonts.status);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        draw_left_text(
            hdc,
            x + 14,
            y + 10 + state.metrics.small + 8,
            w - 24,
            state.metrics.status + 4,
            value,
        );
    }

    unsafe fn paint_settings_chrome(hdc: HDC, state: &UiState, x: i32) {
        let m = state.metrics;
        let s = state.ui_language.strings();
        paint_heading(
            hdc,
            &state.fonts,
            x,
            m.title_bar_h + 16,
            s.settings_header,
            s.settings_sub,
            m,
            560,
            pal(state),
        );
        let title = match state.settings_section {
            SettingsSection::General => s.general,
            SettingsSection::Ai => s.ai_connection,
            SettingsSection::Style => s.commentary_style,
            SettingsSection::Voice => s.voice_audio,
        };
        SelectObject(hdc, state.fonts.heading);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).gold)));
        draw_left_text(
            hdc,
            x + state.settings_nav_w + 24,
            m.page_header_h(),
            480,
            m.heading_h(),
            title,
        );
        if state.settings_section == SettingsSection::Voice {
            let app_label = if state.app_volume_available {
                format!("{}%", state.app_volume)
            } else {
                state.ui_language.strings().app_volume_unavailable.to_string()
            };
            paint_slider_value(hdc, state, state.app_slider_rect, app_label);
            paint_slider_value(
                hdc,
                state,
                state.slider_rect,
                format!("{}%", edit_text(state.volume)),
            );
        }
        if state.settings_section == SettingsSection::Style {
            paint_prompt_counter(hdc, state);
        }
    }

    unsafe fn paint_prompt_counter(hdc: HDC, state: &UiState) {
        let rect = state.prompt_count_rect;
        if rect.right <= rect.left {
            return;
        }
        let count = prompt_text(state.prompt)
            .chars()
            .count()
            .min(MAX_CUSTOM_STYLE_PROMPT_CHARS);
        let s = state.ui_language.strings();
        let label = format!("{count} / {MAX_CUSTOM_STYLE_PROMPT_CHARS} {}", s.prompt_count_suffix);
        let color = if count >= MAX_CUSTOM_STYLE_PROMPT_CHARS {
            pal(state).vermilion
        } else if count * 10 >= MAX_CUSTOM_STYLE_PROMPT_CHARS * 9 {
            pal(state).gold
        } else {
            pal(state).text_muted
        };
        SelectObject(hdc, state.fonts.small);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(to_colorref(color)));
        let mut wide_text = wide(&label);
        let mut text_rect = rect;
        DrawTextW(
            hdc,
            &mut wide_text,
            &mut text_rect,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }

    unsafe fn paint_meta(hdc: HDC, state: &UiState, x: i32, y: i32, label: &str, value: &str) {
        SelectObject(hdc, state.fonts.small);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text_muted)));
        draw_left_text(hdc, x, y, 280, state.metrics.small + 4, label);
        SelectObject(hdc, state.fonts.status);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        let shown = display_value(state, label, value);
        draw_left_text(
            hdc,
            x,
            y + state.metrics.small + 6,
            320,
            state.metrics.status + 8,
            &shown,
        );
    }

    fn display_value(state: &UiState, label: &str, value: &str) -> String {
        let s = state.ui_language.strings();
        if label == s.style || label == "STYLE" {
            state.ui_language.style_title(value).to_string()
        } else if label == s.api || label == "API" {
            state.ui_language.provider_title(value).to_string()
        } else if value.is_empty() {
            "—".to_string()
        } else {
            value.to_string()
        }
    }

    unsafe fn paint_heading(
        hdc: HDC,
        fonts: &Fonts,
        x: i32,
        y: i32,
        title: &str,
        subtitle: &str,
        m: Metrics,
        width: i32,
        palette: Palette,
    ) {
        SelectObject(hdc, fonts.title);
        SetTextColor(hdc, COLORREF(to_colorref(palette.text)));
        draw_left_text(hdc, x, y, width.max(480), m.title + 6, title);
        SelectObject(hdc, fonts.subtitle);
        SetTextColor(hdc, COLORREF(to_colorref(palette.text_muted)));
        draw_left_text(
            hdc,
            x,
            y + m.title + 6,
            width.max(480),
            m.subtitle + 6,
            subtitle,
        );
        let accent_top = y + m.title + 6 + m.subtitle + 10;
        fill_color(
            hdc,
            RECT {
                left: x,
                top: accent_top,
                right: x + 40,
                bottom: accent_top + 1,
            },
            palette.gold,
        );
    }

    unsafe fn paint_slider_value(hdc: HDC, state: &UiState, rect: RECT, value: String) {
        if rect.right <= rect.left {
            return;
        }
        SelectObject(hdc, state.fonts.status);
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        draw_left_text(
            hdc,
            rect.right + 16,
            rect.top - 4,
            if value.len() > 8 { 220 } else { 80 },
            state.metrics.status + 4,
            &value,
        );
    }

    unsafe fn paint_volume_sliders(hdc: HDC, state: &UiState) {
        paint_volume_slider(hdc, state, state.app_slider_rect, state.app_volume as i32);
        let tts_volume = edit_text(state.volume)
            .parse::<i32>()
            .unwrap_or(80)
            .clamp(0, 100);
        paint_volume_slider(hdc, state, state.slider_rect, tts_volume);
    }

    unsafe fn paint_volume_slider(hdc: HDC, state: &UiState, rect: RECT, volume: i32) {
        if rect.right <= rect.left {
            return;
        }
        let volume = volume.clamp(0, 100);
        let y = (rect.top + rect.bottom) / 2;
        fill_color(
            hdc,
            RECT {
                left: rect.left,
                top: y - 2,
                right: rect.right,
                bottom: y + 2,
            },
            pal(state).border,
        );
        let filled = rect.left + ((rect.right - rect.left) * volume) / 100;
        fill_color(
            hdc,
            RECT {
                left: rect.left,
                top: y - 2,
                right: filled,
                bottom: y + 2,
            },
            pal(state).green,
        );
        let thumb = if state.theme == UiTheme::Dark {
            pal(state).paper
        } else {
            pal(state).text
        };
        fill_ellipse(hdc, filled - 7, y - 7, filled + 7, y + 7, thumb);
    }

    unsafe fn fill_color(hdc: HDC, rect: RECT, color: (u8, u8, u8)) {
        let brush = CreateSolidBrush(COLORREF(to_colorref(color)));
        let _ = FillRect(hdc, &rect, brush);
        let _ = DeleteObject(brush);
    }

    unsafe fn fill_round_rect(
        hdc: HDC,
        rect: RECT,
        radius: i32,
        fill: (u8, u8, u8),
        border: Option<(u8, u8, u8)>,
    ) {
        let radius = radius.max(1);
        if let Some(border) = border.filter(|color| *color != fill) {
            let outer = CreateRoundRectRgn(
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                radius,
                radius,
            );
            let border_brush = CreateSolidBrush(COLORREF(to_colorref(border)));
            let _ = FillRgn(hdc, outer, border_brush);
            let _ = DeleteObject(border_brush);
            let inner = CreateRoundRectRgn(
                rect.left + 1,
                rect.top + 1,
                rect.right - 1,
                rect.bottom - 1,
                (radius - 1).max(1),
                (radius - 1).max(1),
            );
            let fill_brush = CreateSolidBrush(COLORREF(to_colorref(fill)));
            let _ = FillRgn(hdc, inner, fill_brush);
            let _ = DeleteObject(fill_brush);
            let _ = DeleteObject(inner);
            let _ = DeleteObject(outer);
        } else {
            let region = CreateRoundRectRgn(
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                radius,
                radius,
            );
            let fill_brush = CreateSolidBrush(COLORREF(to_colorref(fill)));
            let _ = FillRgn(hdc, region, fill_brush);
            let _ = DeleteObject(fill_brush);
            let _ = DeleteObject(region);
        }
    }

    unsafe fn fill_ellipse(
        hdc: HDC,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        color: (u8, u8, u8),
    ) {
        let region = CreateEllipticRgn(left, top, right, bottom);
        let brush = CreateSolidBrush(COLORREF(to_colorref(color)));
        let _ = FillRgn(hdc, region, brush);
        let _ = DeleteObject(brush);
        let _ = DeleteObject(region);
    }

    unsafe fn draw_left_text(hdc: HDC, x: i32, y: i32, w: i32, h: i32, text: &str) {
        let mut wide_text = wide(text);
        let mut rect = RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        DrawTextW(
            hdc,
            &mut wide_text,
            &mut rect,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
        );
    }

    unsafe fn draw_center_text(hdc: HDC, x: i32, y: i32, w: i32, h: i32, text: &str) {
        let mut wide_text = wide(text);
        let mut rect = RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        DrawTextW(
            hdc,
            &mut wide_text,
            &mut rect,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
        );
    }

    fn status_values(state: &UiState) -> (String, String, String, String) {
        let s = state.ui_language.strings();
        let hero = match &state.status_value {
            LauncherStatus::Ready | LauncherStatus::Stopped => s.ready.to_string(),
            LauncherStatus::Starting => s.starting.to_string(),
            LauncherStatus::Running => s.start_running.to_string(),
            LauncherStatus::Stopping => s.stopping.to_string(),
            LauncherStatus::Error(message) => {
                let mapped = state.ui_language.start_error_text(message);
                if mapped.contains("ElevenLabs API Key") {
                    mapped
                } else {
                    s.start_failed.to_string()
                }
            }
        };
        let (ai, obs) = if let Some(session) = &state.session {
            (
                match session.ai_hint() {
                    crate::launcher::AiConnectionHint::Connected => s.connected,
                    crate::launcher::AiConnectionHint::Unavailable => s.unavailable,
                    crate::launcher::AiConnectionHint::Unknown => s.idle,
                },
                match session.obs_hint() {
                    crate::launcher::ObsConnectionHint::Connected => s.connected,
                    crate::launcher::ObsConnectionHint::Unavailable => s.unavailable,
                    crate::launcher::ObsConnectionHint::Unknown => s.idle,
                },
            )
        } else {
            (s.idle, s.idle)
        };
        let style = state
            .ui_language
            .style_title(&unsafe { combo_text(state.style) })
            .to_string();
        (hero, ai.to_string(), obs.to_string(), style)
    }

    unsafe fn paint_field_focus(hwnd: HWND, hdc: HDC) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        if state.page != Page::Settings || state.focused.is_invalid() {
            return;
        };
        for target in form_fields(state) {
            if target.0 != state.focused.0 || !IsWindowVisible(target).as_bool() {
                continue;
            }
            let mut screen = RECT::default();
            let _ = GetWindowRect(target, &mut screen);
            let mut top_left = POINT {
                x: screen.left,
                y: screen.top,
            };
            let mut bottom_right = POINT {
                x: screen.right,
                y: screen.bottom,
            };
            let _ = ScreenToClient(hwnd, &mut top_left);
            let _ = ScreenToClient(hwnd, &mut bottom_right);
            fill_color(
                hdc,
                RECT {
                    left: top_left.x,
                    top: bottom_right.y,
                    right: bottom_right.x,
                    bottom: bottom_right.y + 1,
                },
                pal(state).green,
            );
        }
    }

    unsafe fn color_static(hwnd: HWND, wparam: WPARAM) -> LRESULT {
        let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
        let Some(state) = ui_state(hwnd) else {
            return LRESULT(0);
        };
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text_muted)));
        SetBkColor(hdc, COLORREF(to_colorref(pal(state).bg)));
        SetBkMode(hdc, TRANSPARENT);
        LRESULT(state.bg.0 as isize)
    }

    unsafe fn color_input(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
        let control = HWND(lparam.0 as *mut core::ffi::c_void);
        let Some(state) = ui_state(hwnd) else {
            return LRESULT(0);
        };
        SetTextColor(hdc, COLORREF(to_colorref(pal(state).text)));
        if control.0 == state.prompt.0 {
            SetBkColor(hdc, COLORREF(to_colorref(pal(state).prompt_bg)));
            return LRESULT(state.prompt_bg.0 as isize);
        }
        if control.0 == state.base_url.0
            || control.0 == state.model.0
            || control.0 == state.api_key.0
            || control.0 == state.volume.0
            || control.0 == state.el_api_key.0
            || control.0 == state.el_voice_id.0
            || control.0 == state.el_model.0
        {
            let (focused, hovered) = field_active(state, control);
            let fill = field_fill(pal(state), focused, hovered);
            SetBkColor(hdc, COLORREF(to_colorref(fill)));
            return if focused || hovered {
                LRESULT(state.elevated.0 as isize)
            } else {
                LRESULT(state.surface.0 as isize)
            };
        }
        SetBkColor(hdc, COLORREF(to_colorref(pal(state).surface)));
        LRESULT(state.surface.0 as isize)
    }

    unsafe fn draw_item(hwnd: HWND, lparam: LPARAM) {
        let draw = &*(lparam.0 as *const DRAWITEMSTRUCT);
        let id = draw.CtlID as i32;
        if matches!(
            id,
            IDC_GAME | IDC_PROVIDER | IDC_VOICE | IDC_STYLE | IDC_UI_LANG | IDC_COMMENTARY_LANG
                | IDC_TTS_ENGINE | IDC_EL_VOICE
        ) {
            draw_combo_item(hwnd, draw);
            return;
        }
        draw_owner_button(hwnd, draw);
    }

    unsafe fn draw_combo_item(hwnd: HWND, draw: &DRAWITEMSTRUCT) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let closed = draw.itemState.0 & ODS_COMBOBOXEDIT != 0;
        let selected = draw.itemState.0 & 1 != 0;
        if closed {
            let (focused, hovered) = field_active(state, draw.hwndItem);
            paint_flat_field(
                draw.hDC,
                draw.rcItem,
                pal(state),
                &state.fonts,
                focused,
                hovered,
                false,
            );
        } else {
            let fill = if selected {
                pal(state).nav_active
            } else {
                pal(state).sidebar
            };
            fill_color(draw.hDC, draw.rcItem, fill);
        }
        let raw = combo_item_label(draw.hwndItem, draw.itemID);
        let shown = localize_combo_label(state, draw.hwndItem, &raw);
        SelectObject(draw.hDC, state.fonts.input);
        SetBkMode(draw.hDC, TRANSPARENT);
        SetTextColor(draw.hDC, COLORREF(to_colorref(pal(state).text)));
        let mut text_rect = draw.rcItem;
        text_rect.left += 12;
        text_rect.right -= if closed { 28 } else { 10 };
        let mut wide_text = wide(&shown);
        DrawTextW(
            draw.hDC,
            &mut wide_text,
            &mut text_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
    }

    unsafe fn combo_shown_text(state: &UiState, combo: HWND) -> String {
        localize_combo_label(state, combo, &combo_text(combo))
    }

    unsafe fn combo_item_label(combo: HWND, item_id: u32) -> String {
        if item_id == u32::MAX {
            return combo_text(combo);
        }
        let mut buffer = vec![0u16; 256];
        SendMessageW(
            combo,
            CB_GETLBTEXT,
            WPARAM(item_id as usize),
            LPARAM(buffer.as_mut_ptr() as isize),
        );
        String::from_utf16_lossy(&buffer)
            .trim_matches('\0')
            .trim()
            .to_string()
    }

    fn localize_combo_label(state: &UiState, combo: HWND, text: &str) -> String {
        if combo.0 == state.style.0 {
            state.ui_language.style_title(text).to_string()
        } else if combo.0 == state.provider.0 {
            state.ui_language.provider_title(text).to_string()
        } else if combo.0 == state.ui_lang.0 || combo.0 == state.commentary_lang.0 {
            state.ui_language.language_title(text).to_string()
        } else {
            text.to_string()
        }
    }

    fn is_settings_nav(id: i32) -> bool {
        matches!(
            id,
            IDC_NAV_GENERAL | IDC_NAV_AI | IDC_NAV_STYLE | IDC_NAV_VOICE
        )
    }

    fn settings_nav_section(id: i32) -> Option<SettingsSection> {
        match id {
            IDC_NAV_GENERAL => Some(SettingsSection::General),
            IDC_NAV_AI => Some(SettingsSection::Ai),
            IDC_NAV_STYLE => Some(SettingsSection::Style),
            IDC_NAV_VOICE => Some(SettingsSection::Voice),
            _ => None,
        }
    }

    unsafe fn draw_owner_button(hwnd: HWND, draw: &DRAWITEMSTRUCT) {
        let id = draw.CtlID as i32;
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let locked = (id == IDC_START && !start_action_enabled(&state.status_value))
            || (id == IDC_STOP && !stop_action_enabled(&state.status_value));
        let hot = if locked {
            false
        } else if is_settings_nav(id) {
            state.hover_hwnd.0 == draw.hwndItem.0
        } else {
            let mut cursor = POINT::default();
            let _ = GetCursorPos(&mut cursor);
            let mut screen = RECT::default();
            let _ = GetWindowRect(draw.hwndItem, &mut screen);
            PtInRect(&screen, cursor).as_bool()
        };
        let pressed = if locked || is_settings_nav(id) {
            false
        } else {
            draw.itemState.0 & 1 != 0
        };
        let (fill, border, text, indicator) = button_colors(id, state, hot, pressed);
        if matches!(id, IDC_THEME_DARK | IDC_THEME_LIGHT) {
            draw_theme_tile(draw, state, id, hot);
            return;
        }
        let slot = if matches!(id, IDC_NAV_HOME | IDC_NAV_SETTINGS) {
            pal(state).sidebar
        } else {
            pal(state).bg
        };
        fill_color(draw.hDC, draw.rcItem, slot);
        let border = if border == fill { None } else { Some(border) };
        if fill != slot || border.is_some() {
            fill_round_rect(draw.hDC, draw.rcItem, 5, fill, border);
        }
        if indicator {
            fill_color(
                draw.hDC,
                RECT {
                    left: draw.rcItem.left + 4,
                    top: draw.rcItem.top + 10,
                    right: draw.rcItem.left + 7,
                    bottom: draw.rcItem.bottom - 10,
                },
                pal(state).indicator,
            );
        }
        SetBkMode(draw.hDC, TRANSPARENT);
        SetTextColor(draw.hDC, COLORREF(to_colorref(text)));
        let mut text_rect = draw.rcItem;
        if indicator {
            text_rect.left += 14;
        }
        if (IDC_STYLE_CHIP..IDC_STYLE_CHIP + 5).contains(&id) {
            let key = style_chip_label(id);
            SelectObject(draw.hDC, state.fonts.button);
            let mut title = wide(state.ui_language.style_title(key));
            let mut top = text_rect;
            top.left += 12;
            top.top += 8;
            top.bottom = top.top + state.metrics.button + 2;
            DrawTextW(
                draw.hDC,
                &mut title,
                &mut top,
                DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
            );
            SelectObject(draw.hDC, state.fonts.small);
            SetTextColor(draw.hDC, COLORREF(to_colorref(pal(state).text_muted)));
            let mut sub = wide(state.ui_language.style_sub(key));
            let mut bottom = text_rect;
            bottom.left += 12;
            bottom.top = top.bottom + 2;
            DrawTextW(
                draw.hDC,
                &mut sub,
                &mut bottom,
                DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
            );
            return;
        }
        SelectObject(draw.hDC, state.fonts.button);
        let mut wide_text = wide(button_label(id, state));
        let nav = matches!(
            id,
            IDC_NAV_HOME
                | IDC_NAV_SETTINGS
                | IDC_NAV_GENERAL
                | IDC_NAV_AI
                | IDC_NAV_STYLE
                | IDC_NAV_VOICE
        );
        let align = if nav {
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX
        } else {
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX
        };
        if nav {
            text_rect.left += 12;
        }
        DrawTextW(draw.hDC, &mut wide_text, &mut text_rect, align);
    }

    unsafe fn draw_theme_tile(draw: &DRAWITEMSTRUCT, state: &UiState, id: i32, hot: bool) {
        let preview = if id == IDC_THEME_DARK {
            Palette::dark()
        } else {
            Palette::light()
        };
        let selected = (id == IDC_THEME_DARK && state.theme == UiTheme::Dark)
            || (id == IDC_THEME_LIGHT && state.theme == UiTheme::Light);
        fill_color(draw.hDC, draw.rcItem, pal(state).bg);
        let border = if selected {
            Some(pal(state).green_deep)
        } else if hot {
            Some(pal(state).gold)
        } else {
            Some(preview.border)
        };
        fill_round_rect(draw.hDC, draw.rcItem, 6, preview.surface, border);
        if selected {
            fill_color(
                draw.hDC,
                RECT {
                    left: draw.rcItem.left + 4,
                    top: draw.rcItem.top + 10,
                    right: draw.rcItem.left + 7,
                    bottom: draw.rcItem.bottom - 10,
                },
                pal(state).indicator,
            );
        }
        let swatch_y = draw.rcItem.top + 12;
        let swatch_h = 10;
        let colors = [preview.bg, preview.green, preview.vermilion, preview.gold];
        let mut x = draw.rcItem.left + 16;
        for color in colors {
            fill_color(
                draw.hDC,
                RECT {
                    left: x,
                    top: swatch_y,
                    right: x + 18,
                    bottom: swatch_y + swatch_h,
                },
                color,
            );
            x += 22;
        }
        SelectObject(draw.hDC, state.fonts.button);
        SetBkMode(draw.hDC, TRANSPARENT);
        SetTextColor(draw.hDC, COLORREF(to_colorref(preview.text)));
        let mut label = wide(button_label(id, state));
        let mut text_rect = RECT {
            left: draw.rcItem.left + 14,
            top: swatch_y + swatch_h + 6,
            right: draw.rcItem.right - 10,
            bottom: draw.rcItem.bottom - 8,
        };
        DrawTextW(
            draw.hDC,
            &mut label,
            &mut text_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }

    fn button_colors(
        id: i32,
        state: &UiState,
        hot: bool,
        pressed: bool,
    ) -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8), bool) {
        if id == IDC_START {
            return start_button_colors(state, hot, pressed);
        }
        if id == IDC_STOP {
            return stop_button_colors(state, hot, pressed);
        }
        if matches!(id, IDC_TEST_CONN | IDC_TEST_VOICE) {
            if pressed {
                return (pal(state).hero, pal(state).border, pal(state).nav_text, false);
            }
            if hot {
                return (
                    pal(state).elevated,
                    pal(state).green,
                    pal(state).nav_text,
                    false,
                );
            }
            return (
                pal(state).surface,
                pal(state).border,
                pal(state).nav_text,
                false,
            );
        }
        if id == IDC_EDIT_SETTINGS {
            let text = if hot { pal(state).text } else { pal(state).text_muted };
            return (pal(state).bg, pal(state).bg, text, false);
        }
        if matches!(id, IDC_NAV_HOME | IDC_NAV_SETTINGS) {
            let active = (id == IDC_NAV_HOME && state.page == Page::Home)
                || (id == IDC_NAV_SETTINGS && state.page == Page::Settings);
            if active {
                return (pal(state).nav_active, pal(state).nav_active, pal(state).nav_text, true);
            }
            let text = if hot { pal(state).nav_text } else { pal(state).nav_idle };
            return (pal(state).sidebar, pal(state).sidebar, text, false);
        }
        if is_settings_nav(id) {
            let active = settings_nav_section(id) == Some(state.settings_section);
            if active {
                return (
                    pal(state).nav_active,
                    pal(state).nav_active,
                    pal(state).nav_text,
                    true,
                );
            }
            let text = if hot {
                pal(state).nav_text
            } else {
                pal(state).nav_idle
            };
            return (pal(state).bg, pal(state).bg, text, false);
        }
        if (IDC_STYLE_CHIP..IDC_STYLE_CHIP + 5).contains(&id) {
            let selected = style_chip_label(id) == unsafe { combo_text(state.style) };
            if selected {
                return (pal(state).nav_active, pal(state).green_deep, pal(state).nav_text, false);
            }
            let fill = if hot { pal(state).elevated } else { pal(state).surface };
            return (fill, fill, pal(state).text, false);
        }
        if (IDC_SCALE_CHIP..IDC_SCALE_CHIP + 4).contains(&id) {
            let selected = FONT_SCALES[(id - IDC_SCALE_CHIP) as usize] == state.metrics.scale;
            if selected {
                return (pal(state).nav_active, pal(state).green_deep, pal(state).nav_text, false);
            }
            let fill = if hot { pal(state).elevated } else { pal(state).surface };
            return (fill, fill, pal(state).text_muted, false);
        }
        let fill = if pressed || hot {
            pal(state).nav_active
        } else {
            pal(state).surface
        };
        (fill, pal(state).border, pal(state).text, false)
    }

    fn hero_indicator_color(state: &UiState) -> (u8, u8, u8) {
        match &state.status_value {
            LauncherStatus::Ready => pal(state).green_deep,
            LauncherStatus::Starting | LauncherStatus::Stopping => pal(state).gold,
            LauncherStatus::Running => pal(state).green,
            LauncherStatus::Stopped => pal(state).text_muted,
            LauncherStatus::Error(_) => pal(state).vermilion,
        }
    }

    fn start_button_colors(
        state: &UiState,
        hot: bool,
        pressed: bool,
    ) -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8), bool) {
        match &state.status_value {
            LauncherStatus::Starting => {
                let fill = pal(state).gold;
                (fill, fill, pal(state).text, false)
            }
            LauncherStatus::Running | LauncherStatus::Stopping => {
                (pal(state).green_deep, pal(state).green_deep, pal(state).text, false)
            }
            LauncherStatus::Error(_) | LauncherStatus::Ready | LauncherStatus::Stopped => {
                let fill = if pressed {
                    pal(state).vermilion_deep
                } else if hot {
                    pal(state).vermilion_hover
                } else {
                    pal(state).vermilion
                };
                (fill, fill, pal(state).text, false)
            }
        }
    }

    fn stop_button_colors(
        state: &UiState,
        hot: bool,
        pressed: bool,
    ) -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8), bool) {
        if !stop_action_enabled(&state.status_value) {
            return (
                pal(state).surface,
                pal(state).surface,
                pal(state).text_muted,
                false,
            );
        }
        if matches!(state.status_value, LauncherStatus::Stopping) {
            return (
                pal(state).surface,
                pal(state).border,
                pal(state).text_muted,
                false,
            );
        }
        if pressed {
            return (
                pal(state).vermilion_deep,
                pal(state).vermilion_deep,
                pal(state).text,
                false,
            );
        }
        if hot {
            return (pal(state).vermilion, pal(state).vermilion, pal(state).text, false);
        }
        (
            pal(state).elevated,
            pal(state).green_deep,
            pal(state).text,
            false,
        )
    }

    fn button_label(id: i32, state: &UiState) -> &'static str {
        let s = state.ui_language.strings();
        match id {
            IDC_START => match &state.status_value {
                LauncherStatus::Starting => s.start_starting,
                LauncherStatus::Running | LauncherStatus::Stopping => s.start_running,
                LauncherStatus::Error(_) => s.start_retry,
                LauncherStatus::Ready | LauncherStatus::Stopped => s.start,
            },
            IDC_STOP => {
                if matches!(state.status_value, LauncherStatus::Stopping) {
                    s.stop_stopping
                } else {
                    s.stop
                }
            }
            IDC_TEST_CONN => s.test_conn,
            IDC_TEST_VOICE => s.test_voice,
            IDC_SAVE => s.save,
            IDC_RESET => s.reset,
            IDC_NAV_HOME => s.home,
            IDC_NAV_SETTINGS => s.settings,
            IDC_NAV_GENERAL => s.general,
            IDC_NAV_AI => s.ai_connection,
            IDC_NAV_STYLE => s.commentary_style,
            IDC_NAV_VOICE => s.voice_audio,
            IDC_EDIT_SETTINGS => s.edit_settings,
            IDC_THEME_DARK => s.theme_dark,
            IDC_THEME_LIGHT => s.theme_light,
            id if (IDC_STYLE_CHIP..IDC_STYLE_CHIP + 5).contains(&id) => {
                state.ui_language.style_title(style_chip_label(id))
            }
            id if (IDC_SCALE_CHIP..IDC_SCALE_CHIP + 4).contains(&id) => scale_chip_label(id),
            _ => "",
        }
    }

    fn style_chip_label(id: i32) -> &'static str {
        CommentaryStyle::all()
            .get((id - IDC_STYLE_CHIP) as usize)
            .map(|item| item.label())
            .unwrap_or("")
    }

    fn scale_chip_label(id: i32) -> &'static str {
        match FONT_SCALES[(id - IDC_SCALE_CHIP) as usize] {
            90 => "90%",
            100 => "100%",
            110 => "110%",
            120 => "120%",
            _ => "100%",
        }
    }

    unsafe fn create_controls(hwnd: HWND) -> Result<(), String> {
        let instance = GetModuleHandleW(None).map_err(|error| error.to_string())?;
        let metrics = Metrics::new(FONT_SCALE_DEFAULT);
        let fonts = make_fonts(metrics);
        let combo = WS_CHILD
            | WS_TABSTOP
            | WS_VSCROLL
            | WINDOW_STYLE(
                CBS_DROPDOWNLIST as u32 | CBS_OWNERDRAWFIXED as u32 | CBS_HASSTRINGS as u32,
            );
        let edit = WS_CHILD | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32);

        let game = combo_box(hwnd, instance, combo, IDC_GAME)?;
        let provider = combo_box(hwnd, instance, combo, IDC_PROVIDER)?;
        let base_url = edit_box(hwnd, instance, edit, "", IDC_BASE_URL)?;
        let model = edit_box(hwnd, instance, edit, "", IDC_MODEL)?;
        let api_key = edit_box(
            hwnd,
            instance,
            edit | WINDOW_STYLE(ES_PASSWORD as u32),
            "",
            IDC_API_KEY,
        )?;
        let voice = combo_box(hwnd, instance, combo, IDC_VOICE)?;
        let tts_engine = combo_box(hwnd, instance, combo, IDC_TTS_ENGINE)?;
        let el_api_key = edit_box(
            hwnd,
            instance,
            edit | WINDOW_STYLE(ES_PASSWORD as u32),
            "",
            IDC_EL_API_KEY,
        )?;
        let el_voice = combo_box(hwnd, instance, combo, IDC_EL_VOICE)?;
        let el_voice_id = edit_box(hwnd, instance, edit, "", IDC_EL_VOICE_ID)?;
        let el_model = edit_box(hwnd, instance, edit, "", IDC_EL_MODEL)?;
        let style = combo_box(hwnd, instance, combo, IDC_STYLE)?;
        let volume = edit_box(
            hwnd,
            instance,
            edit | WINDOW_STYLE(ES_NUMBER as u32),
            "80",
            IDC_VOLUME,
        )?;
        let prompt = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("EDIT"),
            &HSTRING::from(""),
            WS_CHILD
                | WS_TABSTOP
                | WS_VSCROLL
                | WINDOW_STYLE(ES_MULTILINE as u32)
                | WINDOW_STYLE(ES_WANTRETURN as u32)
                | WINDOW_STYLE(ES_AUTOVSCROLL as u32),
            0,
            0,
            100,
            40,
            hwnd,
            HMENU(IDC_PROMPT as *mut core::ffi::c_void),
            instance,
            None,
        )
        .map_err(|error| error.to_string())?;
        let _ = SetWindowTheme(prompt, w!(""), w!(""));
        subclass_prompt(prompt);
        SendMessageW(
            prompt,
            EM_SETLIMITTEXT,
            WPARAM(MAX_CUSTOM_STYLE_PROMPT_CHARS),
            LPARAM(0),
        );

        let mut saved = LauncherConfig::load_from_disk();
        if saved.model.trim().is_empty() {
            saved = saved.with_env_defaults();
        }
        let ui_language = saved.ui_language;
        let strings = ui_language.strings();

        add_combo_string(game, GameType::LeagueOfLegends.label());
        SendMessageW(game, CB_SETCURSEL, WPARAM(0), LPARAM(0));
        for item in ConnectionProvider::all() {
            add_combo_string(provider, item.label());
        }
        select_combo(provider, saved.provider.label());
        set_text(base_url, &saved.base_url);
        set_text(model, &saved.model);
        for item in CommentaryStyle::all() {
            add_combo_string(style, item.label());
        }
        select_combo(style, saved.style.label());
        set_text(volume, &saved.volume.to_string());
        set_text(prompt, &saved.custom_style_prompt);

        let voices = sort_voices_for_selector(&list_installed_voices());
        if voices.is_empty() {
            add_combo_string(voice, SYSTEM_DEFAULT_VOICE);
            SendMessageW(voice, CB_SETCURSEL, WPARAM(0), LPARAM(0));
        } else {
            for item in &voices {
                add_combo_string(voice, &voice_selector_label(item));
            }
            let index = saved
                .voice_name
                .as_ref()
                .and_then(|name| voices.iter().position(|item| &item.name == name))
                .unwrap_or(0);
            SendMessageW(voice, CB_SETCURSEL, WPARAM(index), LPARAM(0));
        }

        for item in TtsProvider::all() {
            add_combo_string(tts_engine, item.label());
        }
        select_combo(tts_engine, saved.tts_provider.label());
        set_text(el_voice_id, &saved.elevenlabs_voice_id);
        set_text(el_model, &saved.elevenlabs_model);
        let pending = saved.app_volume.min(100);
        let (app_volume, app_volume_available, app_volume_pending) = match get_app_volume_percent() {
            Ok(volume) => (volume, true, false),
            Err(_) => (pending, false, true),
        };

        let ui_lang = combo_box(hwnd, instance, combo, IDC_UI_LANG)?;
        for item in UiLanguage::all() {
            add_combo_string(ui_lang, item.combo_label());
        }
        select_combo(ui_lang, saved.ui_language.combo_label());
        let commentary_lang = combo_box(hwnd, instance, combo, IDC_COMMENTARY_LANG)?;
        for item in CommentaryLanguage::all() {
            add_combo_string(commentary_lang, item.combo_label());
        }
        select_combo(commentary_lang, saved.commentary_language.combo_label());

        let style_chips = [
            owner_button(hwnd, instance, IDC_STYLE_CHIP)?,
            owner_button(hwnd, instance, IDC_STYLE_CHIP + 1)?,
            owner_button(hwnd, instance, IDC_STYLE_CHIP + 2)?,
            owner_button(hwnd, instance, IDC_STYLE_CHIP + 3)?,
            owner_button(hwnd, instance, IDC_STYLE_CHIP + 4)?,
        ];
        let scale_chips = [
            owner_button(hwnd, instance, IDC_SCALE_CHIP)?,
            owner_button(hwnd, instance, IDC_SCALE_CHIP + 1)?,
            owner_button(hwnd, instance, IDC_SCALE_CHIP + 2)?,
            owner_button(hwnd, instance, IDC_SCALE_CHIP + 3)?,
        ];

        let palette = saved.theme.palette();
        let state = Box::new(UiState {
            sidebar_w: metrics.sidebar_w,
            settings_nav_w: metrics.settings_nav_w,
            metrics,
            palette,
            theme: saved.theme,
            bg: CreateSolidBrush(COLORREF(to_colorref(palette.bg))),
            sidebar: CreateSolidBrush(COLORREF(to_colorref(palette.sidebar))),
            caption: CreateSolidBrush(COLORREF(to_colorref(palette.caption))),
            surface: CreateSolidBrush(COLORREF(to_colorref(palette.surface))),
            elevated: CreateSolidBrush(COLORREF(to_colorref(palette.elevated))),
            prompt_bg: CreateSolidBrush(COLORREF(to_colorref(palette.prompt_bg))),
            fonts,
            page: Page::Home,
            settings_section: SettingsSection::General,
            ui_language,
            commentary_language: saved.commentary_language,
            game,
            provider,
            base_url,
            model,
            api_key,
            voice,
            tts_engine,
            el_api_key,
            el_voice,
            el_voice_id,
            el_model,
            style,
            style_chips,
            scale_chips,
            ui_lang,
            commentary_lang,
            theme_dark: owner_button(hwnd, instance, IDC_THEME_DARK)?,
            theme_light: owner_button(hwnd, instance, IDC_THEME_LIGHT)?,
            volume,
            label_app_volume: label(hwnd, instance, strings.app_volume, 0)?,
            prompt,
            prompt_label: label(hwnd, instance, strings.prompt_title, 0)?,
            prompt_help: label(hwnd, instance, strings.prompt_help, IDC_PROMPT_HELP)?,
            reset: owner_button(hwnd, instance, IDC_RESET)?,
            save: owner_button(hwnd, instance, IDC_SAVE)?,
            test_conn: owner_button(hwnd, instance, IDC_TEST_CONN)?,
            test_voice: owner_button(hwnd, instance, IDC_TEST_VOICE)?,
            start: owner_button(hwnd, instance, IDC_START)?,
            stop: owner_button(hwnd, instance, IDC_STOP)?,
            note: static_text(hwnd, instance, IDC_NOTE)?,
            nav_home: owner_button(hwnd, instance, IDC_NAV_HOME)?,
            nav_settings: owner_button(hwnd, instance, IDC_NAV_SETTINGS)?,
            nav_general: owner_button(hwnd, instance, IDC_NAV_GENERAL)?,
            nav_ai: owner_button(hwnd, instance, IDC_NAV_AI)?,
            nav_style: owner_button(hwnd, instance, IDC_NAV_STYLE)?,
            nav_voice: owner_button(hwnd, instance, IDC_NAV_VOICE)?,
            edit_settings: owner_button(hwnd, instance, IDC_EDIT_SETTINGS)?,
            label_game: label(hwnd, instance, strings.game, 0)?,
            label_language: label(hwnd, instance, strings.language, 0)?,
            label_commentary: label(hwnd, instance, strings.commentary_language, 0)?,
            label_appearance: label(hwnd, instance, strings.appearance, 0)?,
            label_provider: label(hwnd, instance, strings.provider, 0)?,
            label_base_url: label(hwnd, instance, strings.base_url, 0)?,
            label_model: label(hwnd, instance, strings.model, 0)?,
            label_api_key: label(hwnd, instance, strings.api_key, 0)?,
            label_voice: label(hwnd, instance, strings.voice, 0)?,
            label_tts_engine: label(hwnd, instance, strings.tts_engine, 0)?,
            label_el_api_key: label(hwnd, instance, strings.api_key, 0)?,
            label_el_voice_id: label(hwnd, instance, strings.voice_id, 0)?,
            label_el_model: label(hwnd, instance, strings.model, 0)?,
            label_volume: label(hwnd, instance, strings.tts_volume, 0)?,
            label_scale: label(hwnd, instance, strings.interface_scale, 0)?,
            voices,
            el_voices: Vec::new(),
            el_voices_loaded: false,
            status_value: LauncherStatus::Ready,
            session: None,
            note_text: None,
            focused: HWND::default(),
            slider_rect: RECT::default(),
            app_slider_rect: RECT::default(),
            prompt_count_rect: RECT::default(),
            slider_drag: false,
            app_slider_drag: false,
            app_volume,
            app_volume_available,
            app_volume_pending,
            caption_hot: 0,
            hover_hwnd: HWND::default(),
            last_status_sig: String::new(),
            status_pulse: false,
            mem_dc: HDC::default(),
            mem_bitmap: HBITMAP::default(),
            mem_old: HGDIOBJ::default(),
            mem_w: 0,
            mem_h: 0,
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
        SendMessageW(
            hwnd,
            WM_CHANGEUISTATE,
            WPARAM(((UISF_HIDEFOCUS as usize) << 16) | UIS_SET as usize),
            LPARAM(0),
        );
        apply_visual_metrics(hwnd);
        refresh_status_text(hwnd);
        Ok(())
    }

    unsafe fn make_fonts(metrics: Metrics) -> Fonts {
        Fonts {
            title: make_font(metrics.title, FW_SEMIBOLD.0 as i32),
            subtitle: make_font(metrics.subtitle, FW_NORMAL.0 as i32),
            heading: make_font(metrics.section, FW_SEMIBOLD.0 as i32),
            brand: make_font_ex(metrics.brand, FW_SEMIBOLD.0 as i32, CLEARTYPE_QUALITY.0 as i32),
            label: make_font(metrics.label, FW_NORMAL.0 as i32),
            input: make_font(metrics.input, FW_NORMAL.0 as i32),
            button: make_font(metrics.button, FW_SEMIBOLD.0 as i32),
            status: make_font(metrics.status, FW_NORMAL.0 as i32),
            small: make_font(metrics.small, FW_NORMAL.0 as i32),
        }
    }

    unsafe fn destroy_fonts(fonts: &Fonts) {
        let _ = DeleteObject(fonts.title);
        let _ = DeleteObject(fonts.subtitle);
        let _ = DeleteObject(fonts.heading);
        let _ = DeleteObject(fonts.brand);
        let _ = DeleteObject(fonts.label);
        let _ = DeleteObject(fonts.input);
        let _ = DeleteObject(fonts.button);
        let _ = DeleteObject(fonts.status);
        let _ = DeleteObject(fonts.small);
    }

    unsafe fn make_font(size: i32, weight: i32) -> windows::Win32::Graphics::Gdi::HFONT {
        make_font_ex(size, weight, DEFAULT_QUALITY.0 as i32)
    }

    unsafe fn make_font_ex(
        size: i32,
        weight: i32,
        quality: i32,
    ) -> windows::Win32::Graphics::Gdi::HFONT {
        CreateFontW(
            -size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET.0.into(),
            OUT_DEFAULT_PRECIS.0.into(),
            CLIP_DEFAULT_PRECIS.0.into(),
            quality as u32,
            VARIABLE_PITCH.0 as u32,
            w!("Microsoft YaHei UI"),
        )
    }

    unsafe fn label(
        parent: HWND,
        instance: windows::Win32::Foundation::HMODULE,
        text: &str,
        id: i32,
    ) -> Result<HWND, String> {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            &HSTRING::from(text),
            WS_CHILD,
            0,
            0,
            80,
            20,
            parent,
            HMENU(id as *mut core::ffi::c_void),
            instance,
            None,
        )
        .map_err(|error| error.to_string())
    }

    unsafe fn static_text(
        parent: HWND,
        instance: windows::Win32::Foundation::HMODULE,
        id: i32,
    ) -> Result<HWND, String> {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!(""),
            WS_CHILD,
            0,
            0,
            80,
            20,
            parent,
            HMENU(id as *mut core::ffi::c_void),
            instance,
            None,
        )
        .map_err(|error| error.to_string())
    }

    unsafe fn combo_box(
        parent: HWND,
        instance: windows::Win32::Foundation::HMODULE,
        style: WINDOW_STYLE,
        id: i32,
    ) -> Result<HWND, String> {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("COMBOBOX"),
            w!(""),
            style,
            0,
            0,
            120,
            240,
            parent,
            HMENU(id as *mut core::ffi::c_void),
            instance,
            None,
        )
        .map_err(|error| error.to_string())?;
        let _ = SetWindowTheme(hwnd, w!(""), w!(""));
        subclass_combo(hwnd);
        Ok(hwnd)
    }

    unsafe fn edit_box(
        parent: HWND,
        instance: windows::Win32::Foundation::HMODULE,
        style: WINDOW_STYLE,
        text: &str,
        id: i32,
    ) -> Result<HWND, String> {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("EDIT"),
            &HSTRING::from(text),
            style,
            0,
            0,
            120,
            32,
            parent,
            HMENU(id as *mut core::ffi::c_void),
            instance,
            None,
        )
        .map_err(|error| error.to_string())?;
        let _ = SetWindowTheme(hwnd, w!(""), w!(""));
        flatten_form_control(hwnd);
        Ok(hwnd)
    }

    unsafe fn owner_button(
        parent: HWND,
        instance: windows::Win32::Foundation::HMODULE,
        id: i32,
    ) -> Result<HWND, String> {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            0,
            0,
            80,
            32,
            parent,
            HMENU(id as *mut core::ffi::c_void),
            instance,
            None,
        )
        .map_err(|error| error.to_string())?;
        subclass_owner_button(hwnd);
        Ok(hwnd)
    }

    unsafe fn measure_text(
        hdc: HDC,
        font: windows::Win32::Graphics::Gdi::HFONT,
        text: &str,
    ) -> i32 {
        SelectObject(hdc, font);
        let encoded: Vec<u16> = text.encode_utf16().collect();
        let mut size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &encoded, &mut size);
        size.cx
    }

    unsafe fn apply_visual_metrics(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let m = state.metrics;
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        if client.right < 200 || client.bottom < 200 {
            return;
        }
        let s = state.ui_language.strings();
        let hdc = GetDC(hwnd);
        let brand_w = measure_text(hdc, state.fonts.brand, "COMMENTARY")
            .max(measure_text(hdc, state.fonts.brand, s.brand_title));
        let nav_w = measure_text(hdc, state.fonts.button, s.commentary_style)
            .max(measure_text(hdc, state.fonts.button, s.voice_audio));
        let _ = ReleaseDC(hwnd, hdc);
        state.sidebar_w = (brand_w + 56).max(m.sidebar_w).max(210);
        state.settings_nav_w = (nav_w + 36).max(m.settings_nav_w);

        let home = state.page == Page::Home;
        let settings = state.page == Page::Settings;
        let general = settings && state.settings_section == SettingsSection::General;
        let ai = settings && state.settings_section == SettingsSection::Ai;
        let style = settings && state.settings_section == SettingsSection::Style;
        let voice = settings && state.settings_section == SettingsSection::Voice;
        let elevenlabs = combo_text(state.tts_engine) == TtsProvider::ElevenLabs.label();
        let custom = combo_text(state.style) == CommentaryStyle::Custom.label();
        let combo_drop = 240;

        show(state.nav_home, true);
        show(state.nav_settings, true);
        show(state.nav_general, settings);
        show(state.nav_ai, settings);
        show(state.nav_style, settings);
        show(state.nav_voice, settings);
        show(state.edit_settings, home);
        show(state.start, home);
        show(state.stop, home);
        show(state.label_game, general);
        show(state.game, general);
        show(state.label_language, general);
        show(state.ui_lang, general);
        show(state.label_commentary, general);
        show(state.commentary_lang, general);
        show(state.label_appearance, general);
        show(state.theme_dark, general);
        show(state.theme_light, general);
        show(state.label_scale, general);
        for chip in state.scale_chips {
            show(chip, general);
        }
        show(state.label_provider, ai);
        show(state.provider, ai);
        show(state.label_base_url, ai);
        show(state.base_url, ai);
        show(state.label_model, ai);
        show(state.model, ai);
        show(state.label_api_key, ai);
        show(state.api_key, ai);
        show(state.style, false);
        for chip in state.style_chips {
            show(chip, style);
        }
        show(state.prompt_label, style && custom);
        show(state.prompt_help, style && custom);
        show(state.prompt, style && custom);
        show(state.reset, style && custom);
        show(state.save, style && custom);
        show(state.label_tts_engine, voice);
        show(state.tts_engine, voice);
        show(state.label_voice, voice);
        show(state.voice, voice && !elevenlabs);
        show(state.label_el_api_key, voice && elevenlabs);
        show(state.el_api_key, voice && elevenlabs);
        show(state.el_voice, voice && elevenlabs);
        show(state.label_el_voice_id, voice && elevenlabs);
        show(state.el_voice_id, voice && elevenlabs);
        show(state.label_el_model, voice && elevenlabs);
        show(state.el_model, voice && elevenlabs);
        show(state.label_app_volume, voice);
        show(state.label_volume, voice);
        show(state.volume, false);
        show(state.test_conn, home || ai);
        show(state.test_voice, home || voice);
        show(state.note, settings);

        let nav_x = 16;
        let nav_y = m.title_bar_h + 22 + 20 + m.brand * 2 + m.small + 36;
        let nav_w = state.sidebar_w - 32;
        move_hwnd(state.nav_home, nav_x, nav_y, nav_w, m.nav_item_h);
        move_hwnd(
            state.nav_settings,
            nav_x,
            nav_y + m.nav_item_h + 6,
            nav_w,
            m.nav_item_h,
        );

        let content_x = state.sidebar_w + m.pad;
        let content_w = (client.right - content_x - m.pad).max(420);

        if home {
            let hero_y = m.page_header_h();
            let hero_h = (m.start_h + 88).max(128);
            let mods_y = hero_y + hero_h + 14;
            let config_y = mods_y + 68 + 18;
            let pair_h = m.small + 8 + m.status + 12;
            let edit_y = config_y + m.heading_h() + 10 + pair_h * 2 + 4;
            move_hwnd(state.edit_settings, content_x, edit_y, 160, m.button_h);
            let mut actions_y = edit_y + m.button_h + 18;
            let floor = client.bottom - m.pad - m.start_h - 8 - m.button_h;
            if actions_y > floor {
                actions_y = floor.max(edit_y + 8);
            }
            move_hwnd(state.start, content_x, actions_y, m.start_w, m.start_h);
            move_hwnd(
                state.stop,
                content_x + m.start_w + 12,
                actions_y,
                110,
                m.start_h,
            );
            move_hwnd(
                state.test_conn,
                content_x,
                actions_y + m.start_h + 8,
                180,
                m.button_h,
            );
            move_hwnd(
                state.test_voice,
                content_x + 188,
                actions_y + m.start_h + 8,
                140,
                m.button_h,
            );
        }

        if settings {
            let mut section_y = m.page_header_h();
            for item in [
                state.nav_general,
                state.nav_ai,
                state.nav_style,
                state.nav_voice,
            ] {
                move_hwnd(item, content_x, section_y, state.settings_nav_w, m.nav_item_h);
                section_y += m.nav_item_h + 8;
            }
            let field_x = content_x + state.settings_nav_w + 28;
            let field_w = (client.right - field_x - m.pad).max(300);
            let mut y = m.page_header_h() + m.heading_h() + 16;
            if general {
                move_hwnd(state.label_game, field_x, y, field_w, m.label + 4);
                y += m.label + m.label_gap;
                move_hwnd(state.game, field_x, y, field_w.min(480), combo_drop);
                y += m.input_h + m.field_gap + 8;
                move_hwnd(state.label_language, field_x, y, field_w, m.label + 4);
                y += m.label + m.label_gap;
                move_hwnd(state.ui_lang, field_x, y, field_w.min(480), combo_drop);
                y += m.input_h + m.field_gap + 8;
                move_hwnd(state.label_commentary, field_x, y, field_w, m.label + 4);
                y += m.label + m.label_gap;
                move_hwnd(state.commentary_lang, field_x, y, field_w.min(480), combo_drop);
                y += m.input_h + m.field_gap + 8;
                move_hwnd(state.label_appearance, field_x, y, field_w, m.label + 4);
                y += m.label + m.label_gap;
                let tile_w = 148;
                let tile_h = 72;
                move_hwnd(state.theme_dark, field_x, y, tile_w, tile_h);
                move_hwnd(state.theme_light, field_x + tile_w + 12, y, tile_w, tile_h);
                y += tile_h + m.field_gap + 8;
                move_hwnd(state.label_scale, field_x, y, field_w, m.label + 4);
                y += m.label + m.label_gap;
                let chip_w = 84;
                for (index, chip) in state.scale_chips.iter().enumerate() {
                    move_hwnd(
                        *chip,
                        field_x + index as i32 * (chip_w + 10),
                        y,
                        chip_w,
                        m.button_h,
                    );
                }
            }
            if ai {
                for (lab, control, drop) in [
                    (state.label_provider, state.provider, true),
                    (state.label_base_url, state.base_url, false),
                    (state.label_model, state.model, false),
                    (state.label_api_key, state.api_key, false),
                ] {
                    move_hwnd(lab, field_x, y, field_w, m.label + 4);
                    y += m.label + m.label_gap;
                    move_hwnd(
                        control,
                        field_x,
                        y,
                        field_w.min(560),
                        if drop { combo_drop } else { m.input_h },
                    );
                    y += m.input_h + m.field_gap;
                }
                move_hwnd(state.test_conn, field_x, y + 8, 180, m.button_h);
                move_hwnd(
                    state.note,
                    field_x,
                    y + 8 + m.button_h + 10,
                    field_w,
                    m.status + 8,
                );
            }
            if style {
                let chip_w = ((field_w.min(640) - 12) / 3).max(148);
                let chip_h = (m.button_h + 22).max(58);
                for (index, chip) in state.style_chips.iter().enumerate() {
                    let col = (index % 3) as i32;
                    let row = (index / 3) as i32;
                    move_hwnd(
                        *chip,
                        field_x + col * (chip_w + 10),
                        y + row * (chip_h + 10),
                        chip_w,
                        chip_h,
                    );
                }
                y += chip_h * 2 + 28;
                if custom {
                    move_hwnd(state.prompt_label, field_x, y, field_w, m.heading_h());
                    y += m.heading_h() + 4;
                    move_hwnd(state.prompt_help, field_x, y, field_w, m.small + 8);
                    y += m.small + 12;
                    let editor_h = (client.bottom - y - m.button_h - m.pad - 16 - m.small - 12)
                        .max(200)
                        .min(m.prompt_edit_h);
                    let editor_w = field_w.min(680);
                    move_hwnd(state.prompt, field_x, y, editor_w, editor_h);
                    y += editor_h + 6;
                    state.prompt_count_rect = RECT {
                        left: field_x,
                        top: y,
                        right: field_x + editor_w,
                        bottom: y + m.small + 4,
                    };
                    y += m.small + 12;
                    move_hwnd(state.reset, field_x, y, 120, m.button_h);
                    move_hwnd(state.save, field_x + 132, y, 140, m.button_h);
                } else {
                    state.prompt_count_rect = RECT::default();
                }
            }
            if voice {
                if !state.app_slider_drag {
                    let _ = refresh_app_volume_state(state);
                }
                move_hwnd(state.label_tts_engine, field_x, y, field_w, m.label + 4);
                y += m.label + m.label_gap;
                move_hwnd(state.tts_engine, field_x, y, field_w.min(560), combo_drop);
                y += m.input_h + m.field_gap + 8;
                if elevenlabs {
                    move_hwnd(state.label_el_api_key, field_x, y, field_w, m.label + 4);
                    y += m.label + m.label_gap;
                    move_hwnd(state.el_api_key, field_x, y, field_w.min(560), m.input_h);
                    y += m.input_h + m.field_gap;
                    move_hwnd(state.label_voice, field_x, y, field_w, m.label + 4);
                    y += m.label + m.label_gap;
                    move_hwnd(state.el_voice, field_x, y, field_w.min(560), combo_drop);
                    y += m.input_h + m.field_gap;
                    move_hwnd(state.label_el_voice_id, field_x, y, field_w, m.label + 4);
                    y += m.label + m.label_gap;
                    move_hwnd(state.el_voice_id, field_x, y, field_w.min(560), m.input_h);
                    y += m.input_h + m.field_gap;
                    move_hwnd(state.label_el_model, field_x, y, field_w, m.label + 4);
                    y += m.label + m.label_gap;
                    move_hwnd(state.el_model, field_x, y, field_w.min(560), m.input_h);
                    y += m.input_h + m.field_gap;
                } else {
                    move_hwnd(state.label_voice, field_x, y, field_w, m.label + 4);
                    y += m.label + m.label_gap;
                    move_hwnd(state.voice, field_x, y, field_w.min(560), combo_drop);
                    y += m.input_h + m.field_gap + 12;
                }
                move_hwnd(state.label_app_volume, field_x, y, field_w, m.label + 4);
                y += m.label + m.label_gap + 8;
                state.app_slider_rect = RECT {
                    left: field_x,
                    top: y,
                    right: field_x + field_w.min(420),
                    bottom: y + 24,
                };
                y += 40;
                move_hwnd(state.label_volume, field_x, y, field_w, m.label + 4);
                y += m.label + m.label_gap + 8;
                state.slider_rect = RECT {
                    left: field_x,
                    top: y,
                    right: field_x + field_w.min(420),
                    bottom: y + 24,
                };
                y += 48;
                move_hwnd(state.test_voice, field_x, y, 150, m.button_h);
                move_hwnd(state.note, field_x, y + m.button_h + 10, field_w, m.status + 8);
            } else {
                state.slider_rect = RECT::default();
                state.app_slider_rect = RECT::default();
            }
            if !(style && custom) {
                state.prompt_count_rect = RECT::default();
            }
        }

        for combo in [
            state.game,
            state.provider,
            state.voice,
            state.tts_engine,
            state.el_voice,
            state.style,
            state.ui_lang,
            state.commentary_lang,
        ] {
            SendMessageW(
                combo,
                CB_SETITEMHEIGHT,
                WPARAM(usize::MAX),
                LPARAM(m.input_h as isize),
            );
            SendMessageW(
                combo,
                CB_SETITEMHEIGHT,
                WPARAM(0),
                LPARAM((m.input_h - 8) as isize),
            );
            SendMessageW(
                combo,
                WM_SETFONT,
                WPARAM(state.fonts.input.0 as usize),
                LPARAM(1),
            );
        }
        for edit in [
            state.base_url,
            state.model,
            state.api_key,
            state.volume,
            state.prompt,
            state.el_api_key,
            state.el_voice_id,
            state.el_model,
        ] {
            SendMessageW(
                edit,
                WM_SETFONT,
                WPARAM(state.fonts.input.0 as usize),
                LPARAM(1),
            );
            let margin = 12 | (12 << 16);
            SendMessageW(edit, EM_SETMARGINS, WPARAM(EC_BOTH_MARGINS), LPARAM(margin));
        }
        SendMessageW(
            state.note,
            WM_SETFONT,
            WPARAM(state.fonts.status.0 as usize),
            LPARAM(1),
        );
        for item in [
            state.label_game,
            state.label_language,
            state.label_commentary,
            state.label_appearance,
            state.label_provider,
            state.label_base_url,
            state.label_model,
            state.label_api_key,
            state.label_voice,
            state.label_tts_engine,
            state.label_el_api_key,
            state.label_el_voice_id,
            state.label_el_model,
            state.label_volume,
            state.label_app_volume,
            state.label_scale,
            state.prompt_help,
        ] {
            SendMessageW(
                item,
                WM_SETFONT,
                WPARAM(state.fonts.label.0 as usize),
                LPARAM(1),
            );
        }
        SendMessageW(
            state.prompt_label,
            WM_SETFONT,
            WPARAM(state.fonts.heading.0 as usize),
            LPARAM(1),
        );
        let _ = content_w;
        invalidate_settings_nav_buttons(state);
        let _ = InvalidateRect(hwnd, None, false);
    }

    unsafe fn settings_nav_buttons(state: &UiState) -> [HWND; 4] {
        [
            state.nav_general,
            state.nav_ai,
            state.nav_style,
            state.nav_voice,
        ]
    }

    unsafe fn invalidate_settings_nav_buttons(state: &UiState) {
        for button in settings_nav_buttons(state) {
            SendMessageW(button, BM_SETSTATE, WPARAM(0), LPARAM(0));
            let _ = InvalidateRect(button, None, false);
        }
    }

    unsafe fn paint_settings_nav_now(hwnd: HWND) {
        if let Some(state) = ui_state(hwnd) {
            invalidate_settings_nav_buttons(state);
        }
        let _ = InvalidateRect(hwnd, None, false);
        let _ = RedrawWindow(
            hwnd,
            None,
            None,
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN | RDW_NOERASE,
        );
    }

    unsafe fn show(hwnd: HWND, vis: bool) {
        let _ = ShowWindow(hwnd, if vis { SW_SHOW } else { SW_HIDE });
    }

    unsafe fn move_hwnd(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
        let _ = SetWindowPos(hwnd, HWND::default(), x, y, w, h, SWP_NOZORDER);
    }

    unsafe fn set_page(hwnd: HWND, page: Page, section: Option<SettingsSection>) {
        if let Some(state) = ui_state(hwnd) {
            state.page = page;
            if let Some(section) = section {
                state.settings_section = section;
            }
        }
        apply_visual_metrics(hwnd);
        paint_settings_nav_now(hwnd);
        refresh_elevenlabs_voices(hwnd, false);
    }

    unsafe fn set_ui_language(hwnd: HWND, lang: UiLanguage) {
        if let Some(state) = ui_state(hwnd) {
            if state.ui_language == lang {
                return;
            }
            state.ui_language = lang;
        }
        apply_localized_labels(hwnd);
        apply_visual_metrics(hwnd);
        if let Some(state) = ui_state(hwnd) {
            select_combo(state.ui_lang, lang.combo_label());
            match read_config(state) {
                Ok(config) => {
                    let _ = config.save_to_disk();
                }
                Err(_) => {
                    let mut config = LauncherConfig::load_from_disk();
                    config.ui_language = lang;
                    let _ = config.save_to_disk();
                }
            }
        }
    }

    unsafe fn on_ui_language_combo(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let Some(lang) = UiLanguage::from_combo_label(&combo_text(state.ui_lang)) else {
            return;
        };
        set_ui_language(hwnd, lang);
    }

    unsafe fn on_commentary_language_combo(hwnd: HWND) {
        let lang = {
            let Some(state) = ui_state(hwnd) else {
                return;
            };
            let Some(lang) = CommentaryLanguage::from_combo_label(&combo_text(state.commentary_lang))
            else {
                return;
            };
            if state.commentary_language == lang {
                return;
            }
            state.commentary_language = lang;
            lang
        };
        if let Some(state) = ui_state(hwnd) {
            match read_config(state) {
                Ok(config) => {
                    let _ = config.save_to_disk();
                }
                Err(_) => {
                    let mut config = LauncherConfig::load_from_disk();
                    config.commentary_language = lang;
                    let _ = config.save_to_disk();
                }
            }
        }
        let _ = InvalidateRect(hwnd, None, false);
    }

    unsafe fn set_ui_theme(hwnd: HWND, theme: UiTheme) {
        let palette = theme.palette();
        if let Some(state) = ui_state(hwnd) {
            if state.theme == theme {
                return;
            }
            let _ = DeleteObject(state.bg);
            let _ = DeleteObject(state.sidebar);
            let _ = DeleteObject(state.caption);
            let _ = DeleteObject(state.surface);
            let _ = DeleteObject(state.elevated);
            let _ = DeleteObject(state.prompt_bg);
            state.theme = theme;
            state.palette = palette;
            state.bg = CreateSolidBrush(COLORREF(to_colorref(palette.bg)));
            state.sidebar = CreateSolidBrush(COLORREF(to_colorref(palette.sidebar)));
            state.caption = CreateSolidBrush(COLORREF(to_colorref(palette.caption)));
            state.surface = CreateSolidBrush(COLORREF(to_colorref(palette.surface)));
            state.elevated = CreateSolidBrush(COLORREF(to_colorref(palette.elevated)));
            state.prompt_bg = CreateSolidBrush(COLORREF(to_colorref(palette.prompt_bg)));
            match read_config(state) {
                Ok(config) => {
                    let _ = config.save_to_disk();
                }
                Err(_) => {
                    let mut config = LauncherConfig::load_from_disk();
                    config.theme = theme;
                    let _ = config.save_to_disk();
                }
            }
        }
        apply_visual_metrics(hwnd);
        refresh_theme_windows(hwnd);
    }

    unsafe extern "system" fn invalidate_child(hwnd: HWND, _lparam: LPARAM) -> BOOL {
        let _ = InvalidateRect(hwnd, None, false);
        BOOL(1)
    }

    unsafe fn refresh_theme_windows(hwnd: HWND) {
        let _ = InvalidateRect(hwnd, None, false);
        let _ = EnumChildWindows(hwnd, Some(invalidate_child), LPARAM(0));
        let _ = RedrawWindow(
            hwnd,
            None,
            None,
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN | RDW_NOERASE,
        );
    }

    unsafe fn apply_localized_labels(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let s = state.ui_language.strings();
        set_text(state.label_game, s.game);
        set_text(state.label_language, s.language);
        set_text(state.label_commentary, s.commentary_language);
        set_text(state.label_appearance, s.appearance);
        set_text(state.label_scale, s.interface_scale);
        set_text(state.label_provider, s.provider);
        set_text(state.label_base_url, s.base_url);
        set_text(state.label_model, s.model);
        set_text(state.label_api_key, s.api_key);
        set_text(state.label_voice, s.voice);
        set_text(state.label_tts_engine, s.tts_engine);
        set_text(state.label_el_api_key, s.api_key);
        set_text(state.label_el_voice_id, s.voice_id);
        set_text(state.label_el_model, s.model);
        set_text(state.label_volume, s.tts_volume);
        set_text(state.label_app_volume, s.app_volume);
        set_text(state.prompt_label, s.prompt_title);
        set_text(state.prompt_help, s.prompt_help);
    }

    unsafe fn on_style_chip(hwnd: HWND, index: usize) {
        let Some(style) = CommentaryStyle::all().get(index).copied() else {
            return;
        };
        if let Some(state) = ui_state(hwnd) {
            select_combo(state.style, style.label());
        }
        on_style_changed(hwnd);
    }

    unsafe fn set_interface_scale(hwnd: HWND, scale: i32) {
        let (width, height) = {
            let Some(state) = ui_state(hwnd) else {
                return;
            };
            let next = snap_font_scale(scale);
            if next == state.metrics.scale {
                return;
            }
            destroy_fonts(&state.fonts);
            state.metrics = Metrics::new(next);
            state.fonts = make_fonts(state.metrics);
            state.metrics.window_size()
        };
        let _ = SetWindowPos(
            hwnd,
            HWND::default(),
            0,
            0,
            width,
            height,
            SWP_NOZORDER | SWP_NOMOVE,
        );
        apply_visual_metrics(hwnd);
    }

    fn slider_contains(rect: RECT, x: i32, y: i32) -> bool {
        rect.right > rect.left
            && x >= rect.left - 4
            && x <= rect.right + 4
            && y >= rect.top - 8
            && y <= rect.bottom + 8
    }

    fn slider_percent(rect: RECT, x: i32) -> i32 {
        let span = (rect.right - rect.left).max(1);
        (((x - rect.left) * 100) / span).clamp(0, 100)
    }

    unsafe fn refresh_app_volume_state(state: &mut UiState) -> bool {
        match get_app_volume_percent() {
            Ok(volume) => {
                let changed = !state.app_volume_available || state.app_volume != volume;
                state.app_volume = volume;
                state.app_volume_available = true;
                state.app_volume_pending = false;
                changed
            }
            Err(_) => {
                if state.app_volume_available {
                    state.app_volume_available = false;
                    true
                } else {
                    false
                }
            }
        }
    }

    unsafe fn on_slider_mouse(hwnd: HWND, lparam: LPARAM, down: bool) {
        let x = (lparam.0 as i32) & 0xffff;
        let y = ((lparam.0 as i32) >> 16) & 0xffff;
        let x = x as i16 as i32;
        let y = y as i16 as i32;
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        if state.page != Page::Settings || state.settings_section != SettingsSection::Voice {
            return;
        }
        let tts_rect = state.slider_rect;
        let app_rect = state.app_slider_rect;
        if down {
            if slider_contains(app_rect, x, y) {
                state.app_slider_drag = true;
                state.slider_drag = false;
                let _ = SetCapture(hwnd);
            } else if slider_contains(tts_rect, x, y) {
                state.slider_drag = true;
                state.app_slider_drag = false;
                let _ = SetCapture(hwnd);
            } else {
                return;
            }
        }
        if state.app_slider_drag {
            state.app_volume = slider_percent(app_rect, x) as u16;
            if set_app_volume_percent(state.app_volume).is_ok() {
                state.app_volume_available = true;
                state.app_volume_pending = false;
            } else {
                state.app_volume_available = false;
                state.app_volume_pending = true;
            }
            let _ = InvalidateRect(hwnd, None, false);
            return;
        }
        if state.slider_drag {
            set_text(state.volume, &slider_percent(tts_rect, x).to_string());
            let _ = InvalidateRect(hwnd, None, false);
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn add_combo_string(combo: HWND, text: &str) {
        let mut text = wide(text);
        SendMessageW(
            combo,
            CB_ADDSTRING,
            WPARAM(0),
            LPARAM(text.as_mut_ptr() as isize),
        );
    }

    unsafe fn select_combo(combo: HWND, label: &str) {
        let count = SendMessageW(combo, CB_GETCOUNT, WPARAM(0), LPARAM(0)).0;
        for index in 0..count {
            let mut buffer = vec![0u16; 256];
            SendMessageW(
                combo,
                CB_GETLBTEXT,
                WPARAM(index as usize),
                LPARAM(buffer.as_mut_ptr() as isize),
            );
            let item = String::from_utf16_lossy(&buffer)
                .trim_matches('\0')
                .trim()
                .to_string();
            if item == label {
                SendMessageW(combo, CB_SETCURSEL, WPARAM(index as usize), LPARAM(0));
                return;
            }
        }
        SendMessageW(combo, CB_SETCURSEL, WPARAM(0), LPARAM(0));
    }

    unsafe fn combo_text(combo: HWND) -> String {
        let index = SendMessageW(combo, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
        if index < 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; 256];
        SendMessageW(
            combo,
            CB_GETLBTEXT,
            WPARAM(index as usize),
            LPARAM(buffer.as_mut_ptr() as isize),
        );
        String::from_utf16_lossy(&buffer)
            .trim_matches('\0')
            .trim()
            .to_string()
    }

    unsafe fn edit_text(edit: HWND) -> String {
        prompt_text(edit).trim().to_string()
    }

    unsafe fn prompt_text(edit: HWND) -> String {
        let mut buffer = vec![0u16; MAX_CUSTOM_STYLE_PROMPT_CHARS + 64];
        let len = GetWindowTextW(edit, &mut buffer);
        if len <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..len as usize]).to_string()
    }

    unsafe fn set_text(hwnd: HWND, text: &str) {
        let _ = SetWindowTextW(hwnd, &HSTRING::from(text));
    }

    fn nonempty_or_default(value: &str, fallback: &str) -> String {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            fallback.to_string()
        } else {
            trimmed.to_string()
        }
    }

    unsafe fn on_provider_changed(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        if combo_text(state.provider) == ConnectionProvider::OpenRouter.label()
            && edit_text(state.base_url).is_empty()
        {
            set_text(state.base_url, OPENROUTER_BASE_URL);
        }
    }

    unsafe fn on_style_changed(hwnd: HWND) {
        apply_visual_metrics(hwnd);
    }

    unsafe fn on_tts_engine_changed(hwnd: HWND) {
        apply_visual_metrics(hwnd);
        if let Some(state) = ui_state(hwnd) {
            match read_config(state) {
                Ok(config) => {
                    let _ = config.save_to_disk();
                }
                Err(_) => {
                    let mut config = LauncherConfig::load_from_disk();
                    config.tts_provider = TtsProvider::from_label(&combo_text(state.tts_engine))
                        .unwrap_or_default();
                    let _ = config.save_to_disk();
                }
            }
        }
        refresh_elevenlabs_voices(hwnd, true);
    }

    unsafe fn on_elevenlabs_voice_changed(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let voice_id = selected_elevenlabs_dropdown_id(state);
        if !voice_id.is_empty() {
            set_text(state.el_voice_id, &voice_id);
        }
    }

    unsafe fn refresh_elevenlabs_voices(hwnd: HWND, force: bool) {
        let preferred = {
            let Some(state) = ui_state(hwnd) else {
                return;
            };
            if combo_text(state.tts_engine) != TtsProvider::ElevenLabs.label() {
                return;
            }
            if !force && state.el_voices_loaded {
                return;
            }
            edit_text(state.el_voice_id)
        };
        let session_key = {
            let Some(state) = ui_state(hwnd) else {
                return;
            };
            let key = edit_text(state.el_api_key);
            if key.is_empty() {
                None
            } else {
                Some(key)
            }
        };
        match list_elevenlabs_voices(session_key.as_deref()) {
            Ok(voices) => {
                if let Some(state) = ui_state(hwnd) {
                    populate_elevenlabs_voice_combo(state, voices, &preferred);
                }
            }
            Err(_) => {
                if let Some(state) = ui_state(hwnd) {
                    if session_key.is_some() {
                        state.el_voices_loaded = true;
                    }
                }
            }
        }
        let _ = InvalidateRect(hwnd, None, false);
    }

    unsafe fn populate_elevenlabs_voice_combo(
        state: &mut UiState,
        voices: Vec<ElevenLabsVoice>,
        preferred: &str,
    ) {
        SendMessageW(state.el_voice, CB_RESETCONTENT, WPARAM(0), LPARAM(0));
        state.el_voices = voices;
        state.el_voices_loaded = true;
        SendMessageW(
            state.el_voice,
            CB_SETITEMHEIGHT,
            WPARAM(usize::MAX),
            LPARAM(state.metrics.input_h as isize),
        );
        SendMessageW(
            state.el_voice,
            CB_SETITEMHEIGHT,
            WPARAM(0),
            LPARAM((state.metrics.input_h - 8) as isize),
        );
        for voice in &state.el_voices {
            add_combo_string(state.el_voice, &voice.combo_label());
        }
        if state.el_voices.is_empty() {
            return;
        }
        let preferred = preferred.trim();
        let index = state
            .el_voices
            .iter()
            .position(|voice| voice.voice_id == preferred)
            .or_else(|| {
                preferred_free_voice_id(&state.el_voices).and_then(|id| {
                    state
                        .el_voices
                        .iter()
                        .position(|voice| voice.voice_id == id)
                })
            })
            .unwrap_or(0);
        SendMessageW(state.el_voice, CB_SETCURSEL, WPARAM(index), LPARAM(0));
        if let Some(voice) = state.el_voices.get(index) {
            set_text(state.el_voice_id, &voice.voice_id);
        }
    }

    unsafe fn selected_elevenlabs_dropdown_id(state: &UiState) -> String {
        let index = SendMessageW(state.el_voice, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
        if index >= 0 {
            if let Some(voice) = state.el_voices.get(index as usize) {
                return voice.voice_id.clone();
            }
        }
        String::new()
    }

    unsafe fn selected_elevenlabs_voice_id(state: &UiState) -> String {
        let dropdown = selected_elevenlabs_dropdown_id(state);
        if !dropdown.is_empty() {
            return dropdown;
        }
        edit_text(state.el_voice_id).trim().to_string()
    }

    unsafe fn on_start(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        if state
            .session
            .as_ref()
            .is_some_and(|session| !session.has_exited())
        {
            popup(hwnd, state.ui_language.strings().pipeline_busy);
            return;
        }
        if let Some(mut leftover) = state.session.take() {
            let _ = leftover.stop();
        }
        if apply_start(state.status_value.clone()).is_err() {
            return;
        }
        set_status(hwnd, LauncherStatus::Starting, None);
        paint_lifecycle_now(hwnd);
        let api_key = edit_text(state.api_key);
        let config = match read_config(state) {
            Ok(config) => config,
            Err(error) => {
                let display = state.ui_language.start_error_text(&error);
                set_status(hwnd, LauncherStatus::Error(display.clone()), None);
                popup(hwnd, &display);
                return;
            }
        };
        let session_key = if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        };
        let el_api_key = edit_text(state.el_api_key);
        let elevenlabs_key = if el_api_key.is_empty() {
            None
        } else {
            Some(el_api_key)
        };
        match PipelineSession::start(config, session_key, elevenlabs_key) {
            Ok(session) => {
                state.session = Some(session);
                set_status(hwnd, apply_started(LauncherStatus::Starting), None);
            }
            Err(error) => {
                let display = state.ui_language.start_error_text(&error);
                set_status(hwnd, LauncherStatus::Error(display.clone()), None);
                popup(hwnd, &display);
            }
        }
    }

    unsafe fn on_stop(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let Some(session) = state.session.as_mut() else {
            if matches!(
                state.status_value,
                LauncherStatus::Running | LauncherStatus::Starting | LauncherStatus::Stopping
            ) {
                set_status(hwnd, LauncherStatus::Stopped, None);
            }
            return;
        };
        if !stop_action_enabled(&state.status_value) {
            return;
        }
        set_status(hwnd, apply_stop_requested(state.status_value.clone()), None);
        paint_lifecycle_now(hwnd);
        match session.stop() {
            Ok(()) => {
                state.session = None;
                set_status(hwnd, apply_stop(LauncherStatus::Stopping), None);
            }
            Err(error) => {
                set_status(hwnd, LauncherStatus::Error(error.clone()), None);
                popup(hwnd, &error);
            }
        }
    }

    unsafe fn on_test_connection(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let config = match read_config(state) {
            Ok(config) => config,
            Err(error) => {
                popup(hwnd, &error);
                return;
            }
        };
        let api_key = edit_text(state.api_key);
        let session_key = if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        };
        match test_llm_connection(&config, session_key.as_deref()) {
            Ok(message) => set_status(hwnd, state.status_value.clone(), Some(message)),
            Err(error) => {
                set_status(hwnd, LauncherStatus::Error(error.clone()), None);
                popup(hwnd, &error);
            }
        }
    }

    unsafe fn on_test_voice(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let config = match read_config(state) {
            Ok(config) => config,
            Err(error) => {
                popup(hwnd, &error);
                return;
            }
        };
        let result = if config.tts_provider == TtsProvider::ElevenLabs {
            let session_key = edit_text(state.el_api_key);
            play_elevenlabs_test_voice(
                if session_key.is_empty() {
                    None
                } else {
                    Some(session_key.as_str())
                },
                &selected_elevenlabs_voice_id(state),
                &config.elevenlabs_model,
                state.ui_language.elevenlabs_test_voice_text(),
                state.ui_language,
            )
        } else {
            play_test_voice_text(
                config.to_tts_config(),
                state.ui_language.test_voice_text(),
            )
        };
        if let Err(error) = result {
            popup(hwnd, &error);
            return;
        }
        if state.app_volume_pending {
            let percent = state.app_volume.min(100);
            std::thread::spawn(move || {
                let _ = apply_app_volume_when_available(percent, 40, 50);
            });
        }
    }

    unsafe fn on_save(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        match read_config(state) {
            Ok(config) => match config.save_to_disk() {
                Ok(()) => set_status(
                    hwnd,
                    state.status_value.clone(),
                    Some(state.ui_language.strings().saved.to_string()),
                ),
                Err(error) => popup(hwnd, &error),
            },
            Err(error) => popup(hwnd, &error),
        }
    }

    unsafe fn on_reset_prompt(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        set_text(state.prompt, DEFAULT_CUSTOM_STYLE_PROMPT);
        enforce_prompt_limit(hwnd);
    }

    unsafe fn enforce_prompt_limit(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let text = prompt_text(state.prompt);
        let count = text.chars().count();
        if count > MAX_CUSTOM_STYLE_PROMPT_CHARS {
            let truncated: String = text.chars().take(MAX_CUSTOM_STYLE_PROMPT_CHARS).collect();
            set_text(state.prompt, &truncated);
            SendMessageW(
                state.prompt,
                EM_SETSEL,
                WPARAM(usize::MAX),
                LPARAM(-1),
            );
        }
        let rect = state.prompt_count_rect;
        if rect.right > rect.left {
            let _ = InvalidateRect(hwnd, Some(&rect), false);
        } else {
            let _ = InvalidateRect(hwnd, None, false);
        }
    }

    unsafe fn read_config(state: &UiState) -> Result<LauncherConfig, String> {
        let volume = edit_text(state.volume)
            .parse::<i32>()
            .map_err(|_| "volume must be a number".to_string())?;
        let volume = validate_volume(volume)?;
        let style = CommentaryStyle::from_label(&combo_text(state.style))
            .ok_or_else(|| "invalid style".to_string())?;
        let provider = ConnectionProvider::from_label(&combo_text(state.provider))
            .ok_or_else(|| "invalid provider".to_string())?;
        let voice_index = SendMessageW(state.voice, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
        let voice_name = if voice_index >= 0 {
            state
                .voices
                .get(voice_index as usize)
                .map(|voice| voice.name.clone())
        } else {
            None
        };
        let mut config = LauncherConfig {
            game: GameType::LeagueOfLegends,
            provider,
            base_url: edit_text(state.base_url),
            model: edit_text(state.model),
            voice_name,
            style,
            custom_style_prompt: edit_text(state.prompt),
            volume,
            ui_language: state.ui_language,
            commentary_language: state.commentary_language,
            theme: state.theme,
            tts_provider: TtsProvider::from_label(&combo_text(state.tts_engine))
                .unwrap_or_default(),
            elevenlabs_voice_id: selected_elevenlabs_voice_id(state),
            elevenlabs_model: nonempty_or_default(
                &edit_text(state.el_model),
                crate::tts::DEFAULT_ELEVENLABS_MODEL,
            ),
            app_volume: state.app_volume.min(100),
        };
        if style == CommentaryStyle::Custom {
            config.custom_style_prompt =
                crate::launcher::validate_custom_style_prompt(&config.custom_style_prompt)?;
        }
        Ok(config)
    }

    unsafe fn set_status(hwnd: HWND, status: LauncherStatus, note: Option<String>) {
        {
            let Some(state) = ui_state(hwnd) else {
                return;
            };
            state.status_value = status;
            state.note_text = note;
            if !matches!(state.status_value, LauncherStatus::Starting) {
                state.status_pulse = false;
            }
        }
        paint_lifecycle_now(hwnd);
    }

    unsafe fn paint_lifecycle_now(hwnd: HWND) {
        let (start, stop, start_label, stop_label) = {
            let Some(state) = ui_state(hwnd) else {
                return;
            };
            state.last_status_sig.clear();
            (
                state.start,
                state.stop,
                button_label(IDC_START, state),
                button_label(IDC_STOP, state),
            )
        };
        set_text(start, start_label);
        set_text(stop, stop_label);
        refresh_status_text(hwnd);
        let _ = InvalidateRect(start, None, true);
        let _ = InvalidateRect(stop, None, true);
        let _ = InvalidateRect(hwnd, None, false);
        let _ = RedrawWindow(
            hwnd,
            None,
            None,
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN,
        );
        let _ = UpdateWindow(start);
        let _ = UpdateWindow(stop);
        let _ = UpdateWindow(hwnd);
    }

    unsafe fn refresh_status_text(hwnd: HWND) {
        let Some(state) = ui_state(hwnd) else {
            return;
        };
        let (hero, ai, obs, style) = status_values(state);
        let note = state.note_text.clone().unwrap_or_default();
        let tts = state
            .session
            .as_ref()
            .and_then(|session| session.tts_hint().status_line())
            .unwrap_or("");
        let pulse = u8::from(state.status_pulse);
        let status = format!("{:?}", state.status_value);
        let sig = format!("{status}|{hero}|{ai}|{obs}|{style}|{note}|{tts}|{pulse}");
        if state.last_status_sig == sig {
            invalidate_lifecycle_buttons(state);
            return;
        }
        state.last_status_sig = sig;
        set_text(state.note, &note);
        invalidate_lifecycle_buttons(state);
        let _ = InvalidateRect(hwnd, None, false);
    }

    unsafe fn invalidate_lifecycle_buttons(state: &UiState) {
        let _ = InvalidateRect(state.start, None, true);
        let _ = InvalidateRect(state.stop, None, true);
    }

    unsafe fn popup(hwnd: HWND, message: &str) {
        let _ = MessageBoxW(hwnd, &HSTRING::from(message), w!("Launcher"), MB_OK);
    }

    unsafe fn ui_state<'a>(hwnd: HWND) -> Option<&'a mut UiState> {
        let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if raw == 0 {
            None
        } else {
            Some(&mut *(raw as *mut UiState))
        }
    }

    unsafe fn take_state(hwnd: HWND) -> Option<Box<UiState>> {
        let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        if raw == 0 {
            None
        } else {
            Some(Box::from_raw(raw as *mut UiState))
        }
    }
}
