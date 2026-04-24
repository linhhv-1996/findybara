use std::time::Duration;
use tokio::time::sleep;
use reqwest::Client;
use serde_json::json;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_shell::ShellExt;
use std::fs;

pub struct OllamaManager {
    pub port: u16,
    pub model_name: String,
}

impl OllamaManager {
    pub fn new(port: u16, model_name: &str) -> Self {
        Self {
            port,
            model_name: model_name.to_string(),
        }
    }

    fn get_base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn launch_sidecar<R: Runtime>(&self, app: &AppHandle<R>) -> anyhow::Result<()> {
        let host_env = format!("127.0.0.1:{}", self.port);
        let app_dir = app.path().app_data_dir()?;
        let models_dir = app_dir.join("ollama_storage");

        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }

        let sidecar = app
            .shell()
            .sidecar("ollama")
            .map_err(|e| anyhow::anyhow!("Lỗi Sidecar: {}", e))?
            .env("OLLAMA_HOST", &host_env)
            .env("OLLAMA_MODELS", models_dir.to_str().unwrap())
            .args(["serve"]);

        sidecar.spawn()?;
        Ok(())
    }

    pub async fn wait_for_server(&self) -> anyhow::Result<()> {
        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .build()?;
        let url = format!("{}/", self.get_base_url());

        for _ in 0..40 {
            if let Ok(res) = client.get(&url).send().await {
                if res.status().is_success() {
                    return Ok(());
                }
            }
            sleep(Duration::from_millis(500)).await;
        }
        Err(anyhow::anyhow!(
            "Ollama Server không phản hồi sau 20 giây!"
        ))
    }

    // Dùng `ollama create` CLI thay vì HTTP API
    // vì Ollama 0.21+ không cho import local GGUF qua /api/create nữa
    pub async fn create_model_if_not_exists<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        gguf_path: &str,
    ) -> anyhow::Result<()> {
        let client = Client::new();

        // 1. Kiểm tra model đã tồn tại chưa
        let tags_url = format!("{}/api/tags", self.get_base_url());
        let res = client.get(&tags_url).send().await?;
        let tags: serde_json::Value = res.json().await?;

        if let Some(models) = tags["models"].as_array() {
            // starts_with để match cả "findybara-model" lẫn "findybara-model:latest"
            if models.iter().any(|m| {
                m["name"]
                    .as_str()
                    .unwrap_or("")
                    .starts_with(&self.model_name)
            }) {
                println!("✅ Model '{}' đã tồn tại, bỏ qua tạo mới.", self.model_name);
                return Ok(());
            }
        }

        println!(
            "📥 Đang tạo model '{}' từ: {}",
            self.model_name, gguf_path
        );

        // 2. Ghi Modelfile tạm ra đĩa
        let app_dir = app.path().app_data_dir()?;
        let modelfile_path = app_dir.join("Modelfile");
        fs::write(&modelfile_path, format!("FROM {}\n", gguf_path))?;

        // 3. Chạy `ollama create <n> -f <Modelfile>`
        let host_env = format!("127.0.0.1:{}", self.port);
        let models_dir = app_dir.join("ollama_storage");

        let output = app
            .shell()
            .sidecar("ollama")
            .map_err(|e| anyhow::anyhow!("Lỗi sidecar: {}", e))?
            .env("OLLAMA_HOST", &host_env)
            .env("OLLAMA_MODELS", models_dir.to_str().unwrap_or_default())
            .args([
                "create",
                &self.model_name,
                "-f",
                modelfile_path.to_str().unwrap_or_default(),
            ])
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Không chạy được ollama create: {}", e))?;

        // Dọn Modelfile tạm
        let _ = fs::remove_file(&modelfile_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "ollama create thất bại:\nstdout: {}\nstderr: {}",
                stdout,
                stderr
            ));
        }

        println!("✅ Tạo model thành công!");
        Ok(())
    }

    pub async fn generate_text(&self, prompt: &str) -> anyhow::Result<String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        // Dùng /api/chat thay vì /api/generate để:
        // 1. Tránh special token leaking (<|im_start|>, <|endoftext|>...)
        //    vì Ollama tự apply đúng chat template cho từng model
        // 2. Có system prompt để giới hạn độ dài + ngôn ngữ trả lời
        let res = client
            .post(format!("{}/api/chat", self.get_base_url()))
            .json(&json!({
                "model": self.model_name,
                "stream": false,
                "think": false,
                "messages": [
                    {
                        "role": "system",
                        "content": "You are Findybara, a macOS file assistant. 
                                    Answer in 1-3 sentences maximum. Be direct and concise. 
                                    No formatting, no bullet points, no repetition. 
                                    Only answer what is asked."
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "options": {
                    "temperature": 0.1,
                    "num_predict": 512
                }
            }))
            .send()
            .await?;

        let data: serde_json::Value = res.json().await?;

        // /api/chat trả về data["message"]["content"]
        let response = data["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // strip_think_block là safety net nếu model nhỏ vẫn nhét <think> vào content
        Ok(strip_think_block(&response))
    }
}

/// Strip toàn bộ <think>...</think> block khỏi response.
/// Safety net phòng khi model vẫn output thinking dù đã set think:false
fn strip_think_block(text: &str) -> String {
    // Trường hợp 1: có cả <think> lẫn </think>
    if let (Some(start), Some(end)) = (text.find("<think>"), text.find("</think>")) {
        if start < end {
            let before = &text[..start];
            let after = &text[end + "</think>".len()..];
            return format!("{}{}", before, after).trim().to_string();
        }
    }

    // Trường hợp 2: chỉ có </think> (thinking bị cắt ngang)
    if let Some(end) = text.find("</think>") {
        return text[end + "</think>".len()..].trim().to_string();
    }

    // Trường hợp 3: không có think block, trả về nguyên bản
    text.trim().to_string()
}
