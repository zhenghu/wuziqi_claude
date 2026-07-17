//! 通过 OpenRouter Chat Completions API 从战术引擎筛选出的合法点中选择落子。

use crate::{Cell, BOARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::time::Duration;

pub(crate) const CONFIG_PATH: &str = "llm_config.json";
pub(crate) const DEFAULT_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
pub(crate) const DEFAULT_MODEL: &str = "openai/gpt-5-mini";

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct LlmConfig {
    api_key: String,
    api_url: String,
    model: String,
}

pub(crate) struct LlmMove {
    pub(crate) position: (usize, usize),
    model: String,
    provider: Option<String>,
}

impl LlmMove {
    pub(crate) fn route_label(&self) -> String {
        match self.provider.as_deref() {
            Some(provider) if !provider.is_empty() => format!("{} via {provider}", self.model),
            _ => self.model.clone(),
        }
    }
}

impl LlmConfig {
    pub(crate) fn new(api_key: String, api_url: String, model: String) -> Result<Self, String> {
        let api_key = api_key.trim().to_string();
        let api_url = api_url.trim().trim_end_matches('/').to_string();
        let model = model.trim().to_string();
        if api_key.is_empty() {
            return Err("OpenRouter API Key is required".to_string());
        }
        if model.is_empty() {
            return Err("OpenRouter model name is required".to_string());
        }
        if !(api_url.starts_with("https://") || api_url.starts_with("http://")) {
            return Err("API URL must start with http:// or https://".to_string());
        }
        if !api_url.ends_with("/chat/completions") {
            return Err("OpenRouter API URL must end with /chat/completions".to_string());
        }
        Ok(Self {
            api_key,
            api_url,
            model,
        })
    }

    pub(crate) fn load() -> Result<Self, String> {
        let text = std::fs::read_to_string(CONFIG_PATH)
            .map_err(|error| format!("Cannot read {CONFIG_PATH}: {error}"))?;
        let mut raw: Self = serde_json::from_str(&text)
            .map_err(|error| format!("Invalid JSON in {CONFIG_PATH}: {error}"))?;
        let repaired = repair_paste_artifact(&raw.api_key);
        let changed = repaired != raw.api_key;
        raw.api_key = repaired;
        let config = Self::new(raw.api_key, raw.api_url, raw.model)?;
        if changed {
            config.save()?;
        }
        Ok(config)
    }

    pub(crate) fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| format!("Cannot serialize {CONFIG_PATH}: {error}"))?;
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(CONFIG_PATH)
            .map_err(|error| format!("Cannot write {CONFIG_PATH}: {error}"))?;
        file.write_all(json.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|error| format!("Cannot write {CONFIG_PATH}: {error}"))?;
        #[cfg(unix)]
        std::fs::set_permissions(CONFIG_PATH, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Cannot secure {CONFIG_PATH}: {error}"))?;
        Ok(())
    }

    pub(crate) fn api_key(&self) -> &str {
        &self.api_key
    }

    pub(crate) fn api_url(&self) -> &str {
        &self.api_url
    }

    pub(crate) fn model(&self) -> &str {
        &self.model
    }
}

/// 修复旧版配置页在 Cmd+V 后错误追加的单个字符 `v`。
fn repair_paste_artifact(api_key: &str) -> String {
    const PREFIX: &str = "sk-or-v1-";
    let Some(without_v) = api_key.strip_suffix('v') else {
        return api_key.to_string();
    };
    let Some(secret) = without_v.strip_prefix(PREFIX) else {
        return api_key.to_string();
    };
    if secret.len() == 64 && secret.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        without_v.to_string()
    } else {
        api_key.to_string()
    }
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionsRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    max_completion_tokens: u32,
    temperature: f32,
    reasoning: ReasoningConfig,
}

#[derive(Serialize)]
struct ReasoningConfig {
    effort: &'static str,
    exclude: bool,
}

fn board_text(board: &[[Cell; BOARD]; BOARD]) -> String {
    let mut text = String::with_capacity(BOARD * (BOARD + 1));
    for row in board {
        for cell in row {
            text.push(match cell {
                Cell::Empty => '.',
                Cell::Black => 'X',
                Cell::White => 'O',
            });
        }
        text.push('\n');
    }
    text
}

fn response_text(value: &Value) -> Option<String> {
    let content = value
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?;
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }
    content.as_array()?.iter().find_map(|part| {
        part.get("text")?
            .as_str()
            .map(std::string::ToString::to_string)
    })
}

fn api_error_message(value: &Value) -> Option<&str> {
    value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .or_else(|| value.get("message").and_then(Value::as_str))
}

fn parse_move(text: &str) -> Option<(usize, usize)> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        let x = value.get("x")?.as_u64()? as usize;
        let y = value.get("y")?.as_u64()? as usize;
        return Some((x, y));
    }

    let values: Vec<_> = trimmed
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<usize>().ok())
        .collect();
    (values.len() == 2).then(|| (values[0], values[1]))
}

pub(crate) fn request_move(
    config: &LlmConfig,
    board: &[[Cell; BOARD]; BOARD],
    candidates: &[(usize, usize)],
) -> Result<LlmMove, String> {
    if candidates.is_empty() {
        return Err("没有合法候选点".to_string());
    }

    let prompt = format!(
        "你执白棋 O，对手执黑棋 X，坐标从 0 到 14，格式为 (x,y)，左上角是 (0,0)。\n\
         当前棋盘（每行对应 y=0..14）：\n{}\n\
         战术引擎给出的合法候选点：{:?}\n\
         综合进攻、防守、后续威胁和中心控制，选出最佳一手。只能从候选点中选择。",
        board_text(board),
        candidates
    );
    let request = ChatCompletionsRequest {
        model: &config.model,
        messages: vec![
            ChatMessage {
                role: "system",
                content: "你是五子棋专家。只输出一个 JSON 对象，例如 {\"x\":7,\"y\":7}，不要解释。"
                    .to_string(),
            },
            ChatMessage {
                role: "user",
                content: prompt,
            },
        ],
        max_completion_tokens: 1_024,
        temperature: 0.2,
        reasoning: ReasoningConfig {
            effort: "minimal",
            exclude: true,
        },
    };

    let response = match ureq::post(&config.api_url)
        .set("Authorization", &format!("Bearer {}", config.api_key))
        .set("Content-Type", "application/json")
        .set("X-OpenRouter-Title", "Wuziqi")
        .timeout(Duration::from_secs(30))
        .send_json(&request)
    {
        Ok(response) => response,
        Err(ureq::Error::Status(code, response)) => {
            let body = response.into_string().unwrap_or_default();
            let detail = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|value| api_error_message(&value).map(str::to_string))
                .unwrap_or_else(|| "unknown API error".to_string());
            return Err(format!("OpenRouter HTTP {code}: {detail}"));
        }
        Err(error) => return Err(format!("OpenRouter request failed: {error}")),
    };
    let value: Value = response
        .into_json()
        .map_err(|error| format!("OpenRouter returned invalid JSON: {error}"))?;
    if let Some(error) = api_error_message(&value) {
        return Err(format!("OpenRouter error: {error}"));
    }
    let text = response_text(&value).ok_or_else(|| {
        let finish_reason = value
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let reasoning_tokens = value
            .pointer("/usage/completion_tokens_details/reasoning_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        format!(
            "OpenRouter response has no text (finish_reason={finish_reason}, reasoning_tokens={reasoning_tokens})"
        )
    })?;
    let chosen = parse_move(&text).ok_or_else(|| format!("无法解析模型落点: {text}"))?;
    if !candidates.contains(&chosen) {
        return Err(format!("模型返回了候选集外的落点: {chosen:?}"));
    }
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .filter(|model| !model.is_empty())
        .ok_or_else(|| "OpenRouter response is missing the routed model".to_string())?
        .to_string();
    let provider = value
        .get("provider")
        .and_then(Value::as_str)
        .filter(|provider| !provider.is_empty())
        .map(str::to_string);
    Ok(LlmMove {
        position: chosen,
        model,
        provider,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_and_plain_coordinates() {
        assert_eq!(parse_move(r#"{"x":7,"y":8}"#), Some((7, 8)));
        assert_eq!(parse_move("(12, 3)"), Some((12, 3)));
        assert_eq!(parse_move("选择 (12, 3)，因为它最好"), Some((12, 3)));
        assert_eq!(parse_move("7 8 9"), None);
    }

    #[test]
    fn extracts_openrouter_chat_completion_text() {
        let value: Value = serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": "{\"x\":1,\"y\":2}"}}]
        });
        assert_eq!(response_text(&value).as_deref(), Some("{\"x\":1,\"y\":2}"));
    }

    #[test]
    fn labels_the_route_reported_by_openrouter() {
        let routed_move = LlmMove {
            position: (1, 2),
            model: "anthropic/claude-sonnet-4".to_string(),
            provider: Some("Anthropic".to_string()),
        };
        assert_eq!(
            routed_move.route_label(),
            "anthropic/claude-sonnet-4 via Anthropic"
        );
    }

    #[test]
    fn chat_request_limits_reasoning_and_reserves_completion_tokens() {
        let request = ChatCompletionsRequest {
            model: DEFAULT_MODEL,
            messages: vec![],
            max_completion_tokens: 1_024,
            temperature: 0.2,
            reasoning: ReasoningConfig {
                effort: "minimal",
                exclude: true,
            },
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["max_completion_tokens"], 1_024);
        assert_eq!(value["reasoning"]["effort"], "minimal");
        assert_eq!(value["reasoning"]["exclude"], true);
        assert!(value.get("max_tokens").is_none());
    }

    #[test]
    fn parses_json_configuration() {
        let raw = r#"{
            "api_key": "sk-or-test",
            "api_url": "https://openrouter.ai/api/v1/chat/completions",
            "model": "openai/gpt-5-mini"
        }"#;
        let parsed: LlmConfig = serde_json::from_str(raw).unwrap();
        let config = LlmConfig::new(parsed.api_key, parsed.api_url, parsed.model).unwrap();
        assert_eq!(config.api_key(), "sk-or-test");
        assert_eq!(config.api_url(), DEFAULT_API_URL);
        assert_eq!(config.model(), DEFAULT_MODEL);
    }

    #[test]
    fn repairs_only_the_known_cmd_v_paste_artifact() {
        let valid = format!("sk-or-v1-{}", "a".repeat(64));
        assert_eq!(repair_paste_artifact(&(valid.clone() + "v")), valid);
        assert_eq!(repair_paste_artifact("unrelated-keyv"), "unrelated-keyv");
        assert_eq!(repair_paste_artifact("sk-or-v1-shortv"), "sk-or-v1-shortv");
    }

    #[test]
    fn validates_manual_configuration() {
        assert!(LlmConfig::new("key".into(), DEFAULT_API_URL.into(), "model".into()).is_ok());
        assert!(LlmConfig::new("".into(), DEFAULT_API_URL.into(), "model".into()).is_err());
        assert!(LlmConfig::new("key".into(), "not-a-url".into(), "model".into()).is_err());
        assert!(LlmConfig::new(
            "key".into(),
            "https://openrouter.ai/api/v1".into(),
            "model".into()
        )
        .is_err());
        assert!(LlmConfig::new("key".into(), DEFAULT_API_URL.into(), "".into()).is_err());
    }
}
