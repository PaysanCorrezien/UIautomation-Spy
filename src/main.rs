use std::ffi::CString;
use std::sync::Mutex;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{Com::*, LibraryLoader::GetModuleHandleA},
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};

static GLOBAL_ELEMENTS: Mutex<Vec<(RECT, String)>> = Mutex::new(Vec::new());

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

fn print_element_info(element: &IUIAutomationElement, depth: i32) -> Result<()> {
    unsafe {
        let name = element.CurrentName()?;
        let control_type = element.CurrentControlType()?;
        let rect = element.CurrentBoundingRectangle()?;

        // Print indented element information
        println!(
            "{}{} '{}' at ({}, {}, {}, {})",
            "  ".repeat(depth as usize),
            get_control_type_name(control_type),
            name,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom
        );

        // Get children
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

fn collect_element_info(
    element: &IUIAutomationElement,
    elements: &mut Vec<(RECT, String)>,
) -> Result<()> {
    unsafe {
        let name = element.CurrentName()?;
        let control_type = element.CurrentControlType()?;
        let rect = element.CurrentBoundingRectangle()?;

        // Add element if it has valid dimensions
        if rect.right - rect.left > 0 && rect.bottom - rect.top > 0 {
            let element_type = get_control_type_name(control_type);
            elements.push((rect, format!("{} ({})", element_type, name)));
        }

        // Process children
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
        let condition = automation.CreateTrueCondition()?;
        let children = element.FindAll(TreeScope_Children, &condition)?;

        let count = children.Length()?;
        for i in 0..count {
            let child = children.GetElement(i)?;
            collect_element_info(&child, elements)?;
        }

        Ok(())
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

            // Create pens for different UI elements
            let box_pen = CreatePen(PS_SOLID, 2, COLORREF(0x0000FF)); // Red
            let text_color = COLORREF(0x00FF00); // Green text

            SelectObject(hdc, HGDIOBJ(box_pen.0));
            SetBkMode(hdc, BACKGROUND_MODE(1)); // Transparent background
            SetTextColor(hdc, text_color);

            // Safely access the global elements through Mutex
            if let Ok(global_elements) = GLOBAL_ELEMENTS.lock() {
                // Sort elements by size (area) to draw larger boxes first
                let mut sorted_elements = global_elements.clone();
                sorted_elements.sort_by_key(|(rect, _)| {
                    -((rect.right - rect.left) * (rect.bottom - rect.top)) // Negative for descending order
                });

                // Draw boxes and labels for all elements
                for (rect, info) in &sorted_elements {
                    if rect.right - rect.left > 0 && rect.bottom - rect.top > 0 {
                        // Draw rectangle and handle the result
                        let _ = Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);

                        // Calculate label position
                        let mut label_rect = RECT {
                            left: rect.left + 5,
                            right: rect.right - 5,
                            top: if rect.bottom - rect.top > 25 {
                                rect.top + 5
                            } else {
                                rect.top.saturating_sub(20)
                            },
                            bottom: if rect.bottom - rect.top > 25 {
                                rect.top + 25
                            } else {
                                rect.top
                            },
                        };

                        // Create background for text
                        SetBkMode(hdc, BACKGROUND_MODE(2)); // OPAQUE
                        SetBkColor(hdc, COLORREF(0)); // Black background

                        let c_string =
                            CString::new(format!("{} ({},{})", info, rect.left, rect.top)).unwrap();
                        let mut text_bytes = c_string.as_bytes_with_nul().to_vec();

                        DrawTextA(
                            hdc,
                            &mut text_bytes,
                            &mut label_rect,
                            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                        );

                        SetBkMode(hdc, BACKGROUND_MODE(1)); // Back to transparent
                    }
                }
            }

            let _ = DeleteObject(HGDIOBJ(box_pen.0));
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_CHAR => {
            match wparam.0 as u8 as char {
                'q' | 'Q' => {
                    if let Err(e) = DestroyWindow(hwnd) {
                        println!("Error destroying window: {:?}", e);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}

fn create_overlay_window() -> Result<HWND> {
    unsafe {
        let instance = GetModuleHandleA(None)?;

        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: HINSTANCE(instance.0),
            lpszClassName: w!("UIInspectorOverlay"),
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        if atom == 0 {
            return Err(Error::from_win32());
        }

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST,
            w!("UIInspectorOverlay"),
            w!("UI Inspector Overlay"),
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

        if let Err(e) = SetLayeredWindowAttributes(hwnd, COLORREF(0), 180, LWA_ALPHA) {
            println!("Warning: Failed to set window transparency: {:?}", e);
        }

        Ok(hwnd)
    }
}

fn main() -> Result<()> {
    // Get command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <window-title>", args[0]);
        println!("Example: {} \"Calculator\"", args[0]);
        println!("Example: {} \"Task Manager\"", args[0]);
        return Ok(());
    }

    // Join all arguments after the program name to support window titles with spaces
    let window_title = &args[1..].join(" ");

    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        println!("Searching for window with title: {}", window_title);

        // Convert the string to UTF-16
        let window_title_utf16: Vec<u16> = window_title
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let target_window = FindWindowW(None, PCWSTR::from_raw(window_title_utf16.as_ptr()))?;

        if target_window.is_invalid() {
            println!("Window '{}' not found. Make sure the application is running and the window title is exact.", window_title);
            println!("\nTips:");
            println!("- Window titles are case-sensitive");
            println!(
                "- Some applications might have different window titles than their program names"
            );
            println!("- Try running the application first before running this tool");
            return Ok(());
        }

        // Rest of the code remains the same
        println!("UI Elements in {}:", window_title);
        println!("Press 'Q' to quit the overlay");

        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
        let root_element = automation.ElementFromHandle(target_window)?;

        // First print the tree structure
        print_element_info(&root_element, 0)?;

        // Then collect all elements for visualization
        let mut elements = Vec::new();
        collect_element_info(&root_element, &mut elements)?;

        // Safely set the global elements
        if let Ok(mut global_elements) = GLOBAL_ELEMENTS.lock() {
            *global_elements = elements;
        }

        // Create overlay window
        let _overlay_hwnd = create_overlay_window()?;

        // Message loop
        let mut message = MSG::default();
        while GetMessageA(&mut message, None, 0, 0).into() {
            let _ = TranslateMessage(&message);
            let _ = DispatchMessageA(&message);
        }
    }
    Ok(())
}
