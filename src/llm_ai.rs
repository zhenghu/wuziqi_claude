//! 通过 OpenAI Responses API 让大模型从战术引擎筛选出的合法点中选择落子。

use crate::{Cell, BOARD};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

pub(crate) const DEFAULT_API_URL: &str = "https://api.openai.com/v1/responses";
pub(crate) const DEFAULT_MODEL: &str = "gpt-5-mini";

#[derive(Clone)]
pub(crate) struct LlmConfig {
    api_key: String,
    api_url: String,
    model: String,
}

impl LlmConfig {
    pub(crate) fn new(api_key: String, api_url: String, model: String) -> Result<Self, String> {
        let api_key = api_key.trim().to_string();
        let api_url = api_url.trim().to_string();
        let model = model.trim().to_string();
        if api_key.is_empty() {
            return Err("API Key is required".to_string());
        }
        if model.is_empty() {
            return Err("Model name is required".to_string());
        }
        if !(api_url.starts_with("https://") || api_url.starts_with("http://")) {
            return Err("API URL must start with http:// or https://".to_string());
        }
        Ok(Self {
            api_key,
            api_url,
            model,
        })
    }

    pub(crate) fn from_env() -> Result<Self, String> {
        let api_key =
            std::env::var("OPENAI_API_KEY").map_err(|_| "未设置 OPENAI_API_KEY".to_string())?;
        Self::new(
            api_key,
            std::env::var("OPENAI_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string()),
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string()),
        )
    }

    pub(crate) fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Serialize)]
struct ResponsesRequest<'a> {
    model: &'a str,
    instructions: &'a str,
    input: String,
    max_output_tokens: u32,
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

fn response_text(value: &Value) -> Option<&str> {
    value.get("output")?.as_array()?.iter().find_map(|item| {
        item.get("content")?.as_array()?.iter().find_map(|content| {
            (content.get("type")?.as_str()? == "output_text")
                .then(|| content.get("text")?.as_str())
                .flatten()
        })
    })
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
) -> Result<(usize, usize), String> {
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
    let request = ResponsesRequest {
        model: &config.model,
        instructions: "你是五子棋专家。只输出一个 JSON 对象，例如 {\"x\":7,\"y\":7}，不要解释。",
        input: prompt,
        max_output_tokens: 512,
    };

    let response = ureq::post(&config.api_url)
        .set("Authorization", &format!("Bearer {}", config.api_key))
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(20))
        .send_json(&request)
        .map_err(|error| format!("API 请求失败: {error}"))?;
    let value: Value = response
        .into_json()
        .map_err(|error| format!("API 响应不是合法 JSON: {error}"))?;
    let text = response_text(&value).ok_or_else(|| "API 响应中没有文本结果".to_string())?;
    let chosen = parse_move(text).ok_or_else(|| format!("无法解析模型落点: {text}"))?;
    if !candidates.contains(&chosen) {
        return Err(format!("模型返回了候选集外的落点: {chosen:?}"));
    }
    Ok(chosen)
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
    fn extracts_responses_api_output_text() {
        let value: Value = serde_json::json!({
            "output": [{"content": [{"type": "output_text", "text": "{\"x\":1,\"y\":2}"}]}]
        });
        assert_eq!(response_text(&value), Some("{\"x\":1,\"y\":2}"));
    }

    #[test]
    fn validates_manual_configuration() {
        assert!(LlmConfig::new("key".into(), DEFAULT_API_URL.into(), "model".into()).is_ok());
        assert!(LlmConfig::new("".into(), DEFAULT_API_URL.into(), "model".into()).is_err());
        assert!(LlmConfig::new("key".into(), "not-a-url".into(), "model".into()).is_err());
        assert!(LlmConfig::new("key".into(), DEFAULT_API_URL.into(), "".into()).is_err());
    }
}
