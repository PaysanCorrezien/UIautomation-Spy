use windows::{
    core::*, Win32::Foundation::RECT, Win32::System::Com::*, Win32::UI::Accessibility::*,
    Win32::UI::WindowsAndMessaging::*,
};

fn get_control_type_name(type_id: UIA_CONTROLTYPE_ID) -> &'static str {
    match type_id.0 {
        50000 => "Button", // UIA_ButtonControlTypeId
        50004 => "Edit",   // UIA_EditControlTypeId
        50026 => "Group",  // UIA_GroupControlTypeId
        50020 => "Text",   // UIA_TextControlTypeId
        50032 => "Window", // UIA_WindowControlTypeId
        _ => "Other",
    }
}

fn print_element_info(element: &IUIAutomationElement, depth: usize) -> Result<()> {
    unsafe {
        let name = element.CurrentName()?;
        let control_type = element.CurrentControlType()?;
        let indent = "  ".repeat(depth);

        // Get the bounding rectangle
        let rect = element.CurrentBoundingRectangle()?;

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

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let window = FindWindowA(None, s!("Calculator"));

        match window {
            Ok(hwnd) => {
                if hwnd.is_invalid() {
                    println!("Calculator window not found. Please open Calculator first.");
                    return Ok(());
                }

                let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
                let root_element: IUIAutomationElement = automation.ElementFromHandle(hwnd)?;

                println!("UI Elements in Calculator:");
                print_element_info(&root_element, 0)?;
            }
            Err(_) => {
                println!("Calculator window not found. Please open Calculator first.");
            }
        }
    }
    Ok(())
}
