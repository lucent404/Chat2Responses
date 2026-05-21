use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use crate::db::AvailableModel;

const CODEX_CATALOG_TEMPLATE: &str = include_str!("../assets/codex-models.json");

#[derive(Debug, Serialize)]
pub struct CodexCatalogStatus {
    pub source_model_count: usize,
    pub generated_model_count: usize,
}

pub fn catalog_status(generated_models: usize) -> CodexCatalogStatus {
    CodexCatalogStatus {
        source_model_count: source_model_count(),
        generated_model_count: generated_models,
    }
}

pub fn generate_catalog_json(models: &[AvailableModel]) -> Result<String> {
    generate_catalog_json_from_str(CODEX_CATALOG_TEMPLATE, models)
}

fn generate_catalog_json_from_str(template: &str, models: &[AvailableModel]) -> Result<String> {
    let mut catalog = read_source_catalog(template)?;
    merge_available_models(&mut catalog, models)?;
    let mut output = serde_json::to_string_pretty(&catalog)?;
    output.push('\n');
    Ok(output)
}

fn source_model_count() -> usize {
    read_source_catalog(CODEX_CATALOG_TEMPLATE)
        .and_then(|catalog| Ok(models_array(&catalog)?.len()))
        .unwrap_or(0)
}

fn read_source_catalog(template: &str) -> Result<Value> {
    serde_json::from_str(template).context("failed to parse bundled Codex model catalog")
}

fn merge_available_models(catalog: &mut Value, models: &[AvailableModel]) -> Result<()> {
    let template = template_model(catalog)?.clone();
    let catalog_models = models_array_mut(catalog)?;
    for (index, model) in models.iter().enumerate() {
        let generated = model_metadata_from_template(&template, model, index);
        if let Some(existing_index) = catalog_models
            .iter()
            .position(|item| item.get("slug").and_then(Value::as_str) == Some(model.id.as_str()))
        {
            catalog_models[existing_index] = generated;
        } else {
            catalog_models.push(generated);
        }
    }
    Ok(())
}

fn template_model(catalog: &Value) -> Result<&Value> {
    let models = models_array(catalog)?;
    models
        .iter()
        .find(|item| item.get("slug").and_then(Value::as_str) == Some("gpt-5.4"))
        .or_else(|| models.first())
        .ok_or_else(|| anyhow!("Codex model catalog has no models to use as a template"))
}

fn models_array(catalog: &Value) -> Result<&Vec<Value>> {
    catalog
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Codex model catalog must contain a models array"))
}

fn models_array_mut(catalog: &mut Value) -> Result<&mut Vec<Value>> {
    catalog
        .get_mut("models")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow!("Codex model catalog must contain a models array"))
}

fn model_metadata_from_template(template: &Value, model: &AvailableModel, index: usize) -> Value {
    let mut generated = template.clone();
    let context_window = model.context_window.max(1);
    let max_context_window = model.max_context_window.max(context_window);
    let input_modalities = if model.supports_image_input {
        vec!["text", "image"]
    } else {
        vec!["text"]
    };
    merge_object_fields(
        &mut generated,
        json!({
            "slug": model.id,
            "display_name": model.id,
            "description": format!("{} via Chat2Responses", model.owner),
            "default_reasoning_level": null,
            "supported_reasoning_levels": [],
            "supports_reasoning_summaries": model.supports_reasoning_summaries,
            "default_reasoning_summary": "auto",
            "support_verbosity": false,
            "default_verbosity": null,
            "context_window": context_window,
            "max_context_window": max_context_window,
            "auto_compact_token_limit": null,
            "effective_context_window_percent": 95,
            "supports_parallel_tool_calls": model.supports_parallel_tool_calls,
            "input_modalities": input_modalities,
            "supports_image_detail_original": model.supports_image_input,
            "supports_search_tool": false,
            "web_search_tool_type": "text",
            "priority": 100 + index as i64,
            "availability_nux": null,
            "upgrade": null,
            "service_tiers": [],
            "additional_speed_tiers": [],
        }),
    );
    generated
}

fn merge_object_fields(target: &mut Value, updates: Value) {
    let Some(target) = target.as_object_mut() else {
        return;
    };
    let Some(updates) = updates.as_object() else {
        return;
    };
    for (key, value) in updates {
        target.insert(key.clone(), value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str, context_window: i64, max_context_window: i64) -> AvailableModel {
        AvailableModel {
            id: id.to_string(),
            source: "mapping".to_string(),
            owner: "Qwen".to_string(),
            candidate_count: 1,
            context_window,
            max_context_window,
            supports_parallel_tool_calls: true,
            supports_reasoning_summaries: false,
            supports_image_input: false,
        }
    }

    fn source_catalog() -> &'static str {
        r#"{
              "models": [
                {
                  "slug": "gpt-5.5",
                  "display_name": "GPT-5.5",
                  "description": "built in",
                  "base_instructions": "template instructions",
                  "context_window": 272000,
                  "max_context_window": 272000,
                  "auto_compact_token_limit": null,
                  "priority": 1
                },
                {
                  "slug": "gpt-5.4",
                  "display_name": "gpt-5.4",
                  "description": "template",
                  "base_instructions": "template instructions",
                  "context_window": 272000,
                  "max_context_window": 1000000,
                  "auto_compact_token_limit": null,
                  "priority": 2
                }
              ]
            }"#
    }

    #[test]
    fn preserves_built_ins_and_appends_available_models() {
        let output = generate_catalog_json_from_str(
            source_catalog(),
            &[model("Qwen-Plus", 128_000, 128_000)],
        )
        .expect("catalog");
        let parsed: Value = serde_json::from_str(&output).expect("json");
        let models = parsed["models"].as_array().expect("models");
        assert_eq!(models.len(), 3);
        assert!(models.iter().any(|item| item["slug"] == "gpt-5.5"));
        let qwen = models
            .iter()
            .find(|item| item["slug"] == "Qwen-Plus")
            .expect("qwen model");
        assert_eq!(qwen["context_window"], 128_000);
        assert_eq!(qwen["max_context_window"], 128_000);
        assert!(qwen["auto_compact_token_limit"].is_null());
        assert_eq!(qwen["base_instructions"], "template instructions");
    }

    #[test]
    fn image_models_include_image_modality() {
        let mut qwen = model("Qwen-VL", 128_000, 128_000);
        qwen.supports_image_input = true;
        let output = generate_catalog_json_from_str(source_catalog(), &[qwen]).expect("catalog");
        let parsed: Value = serde_json::from_str(&output).expect("json");
        let models = parsed["models"].as_array().expect("models");
        let qwen = models
            .iter()
            .find(|item| item["slug"] == "Qwen-VL")
            .expect("qwen model");
        assert_eq!(qwen["input_modalities"], json!(["text", "image"]));
        assert_eq!(qwen["supports_image_detail_original"], true);
    }

    #[test]
    fn replaces_duplicate_slug_without_duplicating() {
        let output =
            generate_catalog_json_from_str(source_catalog(), &[model("gpt-5.4", 64_000, 96_000)])
                .expect("catalog");
        let parsed: Value = serde_json::from_str(&output).expect("json");
        let models = parsed["models"].as_array().expect("models");
        let matches: Vec<_> = models
            .iter()
            .filter(|item| item["slug"] == "gpt-5.4")
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["context_window"], 64_000);
        assert_eq!(matches[0]["max_context_window"], 96_000);
    }

    #[test]
    fn bundled_catalog_preserves_codex_models() {
        assert!(source_model_count() >= 6);
    }

    #[test]
    fn reports_invalid_models_shape() {
        let err = generate_catalog_json_from_str(r#"{"models":{}}"#, &[])
            .expect_err("invalid shape should fail");
        assert!(err
            .to_string()
            .contains("Codex model catalog must contain a models array"));
    }
}
