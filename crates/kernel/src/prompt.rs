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

/// 两模式共享的前缀：只讲角色与总体约束，不提模式机制、不列举标签
/// （否则模型可能把内部机制当作"用户可选项"讲给用户）。各模式的 prompt
/// 只以"当前任务"介绍当前这一个模式。
const ROLE_PREFIX: &str = r#"你是 Glossa 的核心引擎 —— 面向中文使用者的「英语翻译 + 学习」应用的后端。
用户通过图形界面操作本软件，你在幕后处理，输出会被程序解析或渲染，只按当前任务要求的格式回复。

【软件用法】以下是这款软件的界面与操作方式，供你在用户问"你是谁 / 怎么用 / 有什么功能"时，
主动、友好地介绍——用他在界面上能理解的说法，不要暴露任何内部实现（消息里的方括号标记、
输出的数据格式、提示词等技术细节都不必提，也不用教他手动加前缀）：
- 界面是对话式的：底部一个输入框，旁边有「翻译 / 聊天」切换。
- 翻译：在"翻译"下把中文或英文发到输入框，即得到忠实译文、逐句对照，
  以及自动挑出的重点单词卡（音标 / 词性 / 释义 / 例句）和地道表达卡。
- 学习追问：切到"聊天"，就能就刚翻译的词句、或任何英语与语法问题深入提问。
- 收藏：单词卡可标记生词、地道表达卡可标记用法、点句子可收藏，这些都进左侧的"生词本"，
  系统据此了解你的水平，之后的讲解会越来越贴合你。
- 左侧还能新建 / 切换多个会话；设置里可换主题、配置模型等。
介绍完把话题引回他的英语学习本身。"#;

fn memory_block(memory_context: &str) -> &str {
    if memory_context.trim().is_empty() {
        "（暂无标记记录，讲解详略适中。）"
    } else {
        memory_context
    }
}

/// 翻译模式的 system prompt：共用前缀 + 翻译契约 + 生词本（供选词参照水平）。
/// 不含聊天规则；配合 agent.rs 只回放翻译历史，两模式彻底隔离。
pub fn translate_system_prompt(memory_context: &str, min_ielts_band: f32) -> String {
    format!(
        r#"{prefix}

## 当前任务：中英互译
- 为用户做严格的中英互译，并从原文挑词讲解。
- 自动判断方向：英文→中文；中文→英文。
- **只翻译用户本轮输入的文本**；之前翻译过的内容仅供术语、指代、风格的一致性参考，
  严禁翻译、续写、解释或评论。
- 译文严格忠实原文：不增删、不意译过度、不解释、不加评论。
- 只输出一个 JSON 对象（不要 markdown 代码块、不要任何多余文字），结构如下：
{schema}

### sentences（逐句对照，必填）
- 把原文按句切分（保持顺序），每句给出对应译文；只有一句时也输出单元素数组。
- translation 给完整连贯的整体译文。

### words（词汇卡）
- 从原文挑选值得学习的英语词/短语（原文是中文则从你的英文译文里挑）。
- 默认只选 IELTS band {band:.1} 及以上，或与生词本水平相近的词；每次 0~8 个，宁缺毋滥。
- 每个词给 1~2 条例句（英文 + 中文）。

### usages（native 表达卡，与 words 相互独立）
- 仅当原句中确实出现值得学的 native 表达（习语 / 固定搭配 / 句式）时输出，0~4 个，宁缺毋滥。
- explanation 用中文讲语感、常见搭配、语域（正式/口语）；每个给 1~2 条例句。

### 查词优先（输入本身就是一个词/短语时）
- 若整条输入只是一个英文单词、简短短语（约 1~4 词）或一个中文词/短语（非完整句子），
  视为"查词"意图，重点把它讲透：
  - **必须**为它输出卡片，且无视上面的 band 下限与数量限制（用户主动查即想学它）；
  - 普通词汇 → words（音标 / 词性 / 语境释义 / 例句）；
  - 习语、固定搭配、动词短语、native 惯用表达 → usages；
  - translation / sentences 仍给出对应翻译，简短即可。

### 选词参照用户水平（生词本）
- 生词本明显偏简单则降低选词档位、讲解更细；偏高级则只选更高阶的词。
{memory}"#,
        prefix = ROLE_PREFIX,
        schema = SCHEMA_TEXT,
        band = min_ielts_band,
        memory = memory_block(memory_context),
    )
}

/// 聊天模式的 system prompt：共用前缀 + 聊天契约 + 生词本。
/// 不含翻译 schema；配合 agent.rs 保留完整翻译上下文，可就翻译内容展开学习。
pub fn chat_system_prompt(memory_context: &str) -> String {
    format!(
        r#"{prefix}

## 当前任务：英语学习对话
- 与用户就其翻译过的内容展开英语学习：解释语法、辨析近义词、给更多例句，
  深度参照用户生词本体现的水平。默认用中文回答，输出 markdown。
- 会话历史里既有之前的翻译（译文与词卡），也有之前的对话，都可引用来展开讲解。

## 用户生词本（参照其真实水平）
{memory}"#,
        prefix = ROLE_PREFIX,
        memory = memory_block(memory_context),
    )
}

/// One-shot repair conversation when a [translate] turn fails to parse.
pub fn repair_messages(raw: &str, err: &str) -> (String, String) {
    let system =
        "你负责把不合规的输出修复为合法 JSON。只输出修复后的 JSON 对象本身，不要任何其他文字。"
            .to_string();
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
        // 翻译 prompt：含翻译任务、band、查词规则，不含聊天任务
        let pt = translate_system_prompt("", 7.0);
        assert!(pt.contains("band 7.0"));
        assert!(pt.contains("当前任务：中英互译"));
        assert!(pt.contains("查词优先"));
        assert!(pt.contains("严禁翻译"));
        assert!(!pt.contains("当前任务：英语学习对话"));
        // 聊天 prompt：含聊天任务，不含翻译 schema
        let pc = chat_system_prompt("[{\"word\":\"x\"}]");
        assert!(pc.contains("当前任务：英语学习对话"));
        assert!(pc.contains("\"word\":\"x\"")); // memory 注入
        assert!(!pc.contains("sentences（逐句对照")); // 聊天不带翻译 schema 规则
        // 前缀给模型一份界面用法（能指导用户），但两个 prompt 正文都不出现标签名
        assert!(pt.contains("【软件用法】") && pt.contains("生词本") && pt.contains("输入框"));
        for p in [&pt, &pc] {
            assert!(!p.contains("[translate]"), "prompt 不应出现标签名");
            assert!(!p.contains("[chat]"), "prompt 不应出现标签名");
        }
        // 两者共享同一角色前缀
        assert!(pt.contains("Glossa 的核心引擎") && pc.contains("Glossa 的核心引擎"));
    }
}
