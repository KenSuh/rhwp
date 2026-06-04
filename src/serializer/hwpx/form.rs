//! HWPX 양식 컨트롤 직렬화 — 버튼, 체크박스, 라디오, 콤보박스, 입력 상자 보존.

use std::io::Write;

use quick_xml::Writer;

use crate::model::control::{FormObject, FormType};

use super::utils::{empty_tag, end_tag, start_tag_attrs, text};
use super::SerializeError;

pub fn write_form<W: Write>(w: &mut Writer<W>, form: &FormObject) -> Result<(), SerializeError> {
    let tag = form_tag_name(form.form_type);
    let fore_color = color_hex(form.fore_color);
    let back_color = color_hex(form.back_color);
    let enabled = bool01(form.enabled);
    let value = if form.value != 0 {
        "CHECKED"
    } else {
        "UNCHECKED"
    };

    let mut attrs = vec![
        ("name", form.name.as_str()),
        ("foreColor", fore_color.as_str()),
        ("backColor", back_color.as_str()),
        ("enabled", enabled),
    ];
    match form.form_type {
        FormType::PushButton | FormType::CheckBox | FormType::RadioButton => {
            attrs.push(("caption", form.caption.as_str()));
            if matches!(form.form_type, FormType::CheckBox | FormType::RadioButton) {
                attrs.push(("value", value));
            }
        }
        FormType::ComboBox => {
            attrs.push(("selectedValue", form.text.as_str()));
        }
        FormType::Edit => {}
    }

    start_tag_attrs(w, tag, &attrs)?;

    if form.width > 0 || form.height > 0 {
        let width = form.width.to_string();
        let height = form.height.to_string();
        empty_tag(w, "hp:sz", &[("width", &width), ("height", &height)])?;
    }

    if matches!(form.form_type, FormType::ComboBox) {
        for item in sorted_list_items(form) {
            empty_tag(w, "hp:listItem", &[("value", item.as_str())])?;
        }
    }

    if matches!(form.form_type, FormType::Edit) && !form.text.is_empty() {
        start_tag_attrs(w, "hp:text", &[])?;
        text(w, &form.text)?;
        end_tag(w, "hp:text")?;
    }

    end_tag(w, tag)?;
    Ok(())
}

fn form_tag_name(form_type: FormType) -> &'static str {
    match form_type {
        FormType::PushButton => "hp:btn",
        FormType::CheckBox => "hp:checkBtn",
        FormType::ComboBox => "hp:comboBox",
        FormType::RadioButton => "hp:radioBtn",
        FormType::Edit => "hp:edit",
    }
}

fn bool01(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn color_hex(color: u32) -> String {
    if color == 0xFFFF_FFFF {
        return "none".to_string();
    }
    let r = (color & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = ((color >> 16) & 0xFF) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn sorted_list_items(form: &FormObject) -> Vec<String> {
    let mut items: Vec<(usize, String)> = form
        .properties
        .iter()
        .filter_map(|(key, value)| {
            let suffix = key.strip_prefix("listItem")?;
            let index = suffix.parse::<usize>().ok()?;
            Some((index, value.clone()))
        })
        .collect();
    items.sort_by_key(|(index, _)| *index);
    items.into_iter().map(|(_, value)| value).collect()
}
