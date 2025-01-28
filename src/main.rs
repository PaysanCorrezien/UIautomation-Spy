use std::ffi::CString;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{Com::*, LibraryLoader::GetModuleHandleA},
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};

const WINDOW_CLASS_NAME: PCSTR = windows::core::s!("UIInspectorOverlay");

static mut GLOBAL_ELEMENTS: Vec<(RECT, String)> = Vec::new();

fn get_control_type_name(type_id: UIA_CONTROLTYPE_ID) -> &'static str {
    match type_id.0 {
        50000 => "Button",
        50004 => "Edit",
        50026 => "Group",
        50020 => "Text",
        50032 => "Window",
        50003 => "ComboBox",
        50006 => "List",
        50007 => "ListItem",
        50008 => "Menu",
        50009 => "MenuBar",
        50010 => "MenuItem",
        50011 => "ProgressBar",
        50012 => "RadioButton",
        50013 => "ScrollBar",
        50014 => "Slider",
        50015 => "Spinner",
        50016 => "StatusBar",
        50018 => "Tab",
        50019 => "TabItem",
        50025 => "TreeItem",
        50023 => "ToolBar",
        50024 => "ToolTip",
        50033 => "Hyperlink",
        50034 => "CheckBox",
        _ => "Other",
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let brush = CreateSolidBrush(COLORREF(0x0000FF));
            let pen = CreatePen(PS_SOLID, 2, COLORREF(0x0000FF));

            SelectObject(hdc, HGDIOBJ(pen.0));
            SetBkMode(hdc, BACKGROUND_MODE(1));
            SetTextColor(hdc, COLORREF(0xFFFFFF));

            for (rect, info) in &GLOBAL_ELEMENTS {
                Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);

                let mut draw_rect = *rect;
                let c_string = CString::new(info.as_str()).unwrap();
                let mut text_bytes = c_string.as_bytes_with_nul().to_vec();
                DrawTextA(
                    hdc,
                    &mut text_bytes,
                    &mut draw_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
            }

            DeleteObject(HGDIOBJ(brush.0));
            DeleteObject(HGDIOBJ(pen.0));
            EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}

fn print_element_info(element: &IUIAutomationElement, depth: usize) -> Result<()> {
    unsafe {
        let name = element.CurrentName()?;
        let control_type = element.CurrentControlType()?;
        let rect = element.CurrentBoundingRectangle()?;
        let indent = "  ".repeat(depth);

        println!(
            "{}Type: {} ({}), Name: {}",
            indent,
            get_control_type_name(control_type),
            control_type.0,
            name
        );
        println!(
            "{}Position: x={}-{}, y={}-{} ({}x{})",
            indent,
            rect.left,
            rect.right,
            rect.top,
            rect.bottom,
            rect.right - rect.left,
            rect.bottom - rect.top
        );

        // Store element info for visual overlay
        GLOBAL_ELEMENTS.push((
            rect,
            format!("{} ({})", get_control_type_name(control_type), name),
        ));

        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
        let condition = automation.CreateTrueCondition()?;
        let children = element.FindAll(TreeScope_Children, &condition)?;

        let count = children.Length()?;
        for i in 0..count {
            let child = children.GetElement(i)?;
            print_element_info(&child, depth + 1)?;
        }

        Ok(())
    }
}

fn create_overlay_window() -> Result<HWND> {
    unsafe {
        let instance = GetModuleHandleA(None)?;

        let wc = WNDCLASSA {
            lpfnWndProc: Some(window_proc),
            hInstance: HINSTANCE(instance.0),
            lpszClassName: WINDOW_CLASS_NAME,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };

        let atom = RegisterClassA(&wc);
        if atom == 0 {
            return Err(Error::from_win32());
        }

        let hwnd = CreateWindowExA(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST,
            WINDOW_CLASS_NAME,
            windows::core::s!("UI Inspector Overlay"),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
            None,
            None,
            Some(HINSTANCE(instance.0)),
            None,
        )?;

        SetLayeredWindowAttributes(hwnd, COLORREF(0), 128, LWA_ALPHA);

        Ok(hwnd)
    }
}

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let target_window = FindWindowA(None, windows::core::s!("Calculator"))?;

        if target_window.is_invalid() {
            println!("Calculator window not found. Please open Calculator first.");
            return Ok(());
        }

        println!("UI Elements in Calculator:");

        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
        let root_element = automation.ElementFromHandle(target_window)?;

        // Print tree structure and collect elements
        print_element_info(&root_element, 0)?;

        // Create overlay window
        let _overlay_hwnd = create_overlay_window()?;

        // Message loop
        let mut message = MSG::default();
        while GetMessageA(&mut message, None, 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageA(&message);
        }
    }
    Ok(())
}
