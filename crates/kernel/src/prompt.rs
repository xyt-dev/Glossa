use crate::schema::SCHEMA_TEXT;
use crate::Mode;

/// Tag prepended to every user turn so the model always knows which
/// output contract applies, both for the current turn and in replayed history.
pub fn tag_user_text(mode: Mode, text: &str) -> String {
    match mode {
        Mode::Translate => format!("[translate]\n{text}"),
        Mode::Chat => format!("[chat]\n{text}"),
    }
}

/// Global system prompt: role, both output contracts, word-selection rules,
/// and the user's vocab memory for level calibration.
pub fn system_prompt(memory_context: &str, min_ielts_band: f32) -> String {
    format!(
        r#"你是一个专业的中英互译引擎兼英语私教。用户消息带有模式标签，你必须严格按标签对应的输出契约回复。

## [translate] 严格翻译模式
- 自动判断方向：英文→翻译成中文；中文→翻译成英文。
- 译文必须严格忠实原文，不增删、不意译过度、不解释。
- 只输出一个 json 对象（不要 markdown 代码块、不要任何多余文字），结构如下：
{schema}

### 选词规则（words 字段）
- 从**原文**中挑选值得学习的英语词汇/短语（若原文是中文，则从你的英文译文中挑选）。
- 默认只选 ielts band {band:.1} 及以上的词，或与用户生词本中词汇水平相近的词。
- 每次 0~8 个，宁缺毋滥；简单句可以返回空数组。
- native_usage 用中文讲解 native speaker 的真实用法、常见搭配、语域（正式/口语）。
- 用户生词本里反复标记的词说明其真实水平，选词难度要与之匹配：
  生词本明显偏简单则降低选词档位并讲解更细，生词本高级则只选更高阶的词。

## [chat] 聊天模式
- 自由对话，默认用中文回答，输出 markdown。
- 这是英语学习场景：可以引用本会话之前的翻译和词卡展开讲解，
  解释语法、辨析近义词、给更多例句，深度参照用户生词本体现的水平。

## 用户生词本（memory）
{memory}"#,
        schema = SCHEMA_TEXT,
        band = min_ielts_band,
        memory = if memory_context.trim().is_empty() { "（暂无标记记录，按 band 下限选词，讲解详略适中。）" } else { memory_context },
    )
}

/// One-shot repair conversation when a [translate] turn fails to parse.
pub fn repair_messages(raw: &str, err: &str) -> (String, String) {
    let system = "你负责把不合规的输出修复为合法 JSON。只输出修复后的 JSON 对象本身，不要任何其他文字。".to_string();
    let user = format!(
        "下面这段输出应当是符合此结构的 JSON：\n{SCHEMA_TEXT}\n\n但解析失败了（错误：{err}）。请修复并只输出 JSON：\n\n{raw}"
    );
    (system, user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_and_prompt_contain_key_parts() {
        assert!(tag_user_text(Mode::Translate, "hi").starts_with("[translate]"));
        assert!(tag_user_text(Mode::Chat, "hi").starts_with("[chat]"));
        let p = system_prompt("", 7.0);
        assert!(p.contains("band 7.0"));
        assert!(p.contains("translation"));
        let p2 = system_prompt("[{\"word\":\"x\"}]", 7.5);
        assert!(p2.contains("\"word\":\"x\""));
    }
}
