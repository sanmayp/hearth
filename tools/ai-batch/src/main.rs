//! ai-batch binary. It runs public text through model endpoints.
//! Live needs keys plus caps. Dry-run prints the plan first.

use ai_batch::{
    chunk_text, estimate_tokens, generate_report, parse_args, plan_calls, read_input_dir,
    run_batch, BothMode, Command, Provider, ProviderClient, Task,
};

#[tokio::main]
async fn main() {
    let argv: Vec<String> = std::env::args().collect();
    match parse_args(&argv) {
        Ok(Command::Report { usage, out }) => {
            if let Err(e) = generate_report(&usage, &out) {
                eprintln!("report failed: {e}");
                std::process::exit(1);
            }
            println!("report wrote {}", out.display());
        }
        Ok(Command::Run { config, dry_run }) => {
            std::process::exit(run(config, dry_run).await);
        }
        Err(e) => {
            eprintln!("ai-batch: {e}");
            eprintln!("Try: ai-batch run --input=DIR --provider=mock --task=summary");
            std::process::exit(1);
        }
    }
}

/// Run or dry-run. Returns the exit code. 0 includes cap stops.
async fn run(config: ai_batch::Config, dry_run: bool) -> i32 {
    let files = match read_input_dir(&config.input) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("ai-batch: {e}");
            return 1;
        }
    };
    let mut chunks = vec![];
    for (file, text) in &files {
        chunks.extend(chunk_text(file, text));
    }
    if chunks.is_empty() {
        eprintln!("ai-batch: no text chunks found");
        return 1;
    }
    let plan = plan_calls(&chunks, &config);
    let est_tokens: u64 = chunks.iter().map(|c| estimate_tokens(&c.text)).sum::<u64>()
        * requests_per_chunk(&config) as u64;
    if dry_run {
        print_plan(&config, &files, &chunks, &plan, est_tokens);
        return 0;
    }
    let mock_only = config.provider == Provider::Mock;
    let (baseten, modal) = match build_clients(&config) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("ai-batch: {e}");
            return 1;
        }
    };
    if !mock_only {
        println!(
            "live run: provider={:?} task={:?} chunks={} cap_calls={} cap_tokens={}",
            config.provider,
            task_name(config.task),
            chunks.len(),
            cap_word(config.max_calls),
            cap_word(config.max_tokens)
        );
    }
    match run_batch(&config, chunks, baseten, modal).await {
        Ok(summary) => {
            println!(
                "done: planned={} ran={} ok={} errors={} skipped_cap={} tokens={}+{} capped={}",
                summary.planned,
                summary.ran,
                summary.ok,
                summary.errors,
                summary.skipped_cap,
                summary.tokens_in,
                summary.tokens_out,
                summary.capped
            );
            if summary.capped {
                println!("CAP STOP: a cap ended the run early. Results stay partial.");
            }
            0
        }
        Err(e) => {
            eprintln!("ai-batch: {e}");
            2
        }
    }
}

/// Requests one chunk spends.
fn requests_per_chunk(config: &ai_batch::Config) -> usize {
    if config.provider == Provider::Both && config.both == BothMode::Race {
        2
    } else {
        1
    }
}

/// Cap word for logs. Zero means uncapped.
fn cap_word(value: u64) -> String {
    if value == 0 {
        "none".to_string()
    } else {
        value.to_string()
    }
}

/// Task word for logs.
fn task_name(task: Task) -> &'static str {
    match task {
        Task::Summary => "summary",
        Task::Explain => "explain",
        Task::Suppliers => "suppliers",
        Task::Digest => "digest",
    }
}

/// Print the dry-run plan. No calls, no spend.
fn print_plan(
    config: &ai_batch::Config,
    files: &[(String, String)],
    chunks: &[ai_batch::Chunk],
    plan: &[ai_batch::PlannedCall],
    est_tokens: u64,
) {
    println!("dry-run: no calls made, no spend");
    println!(
        "files={} chunks={} calls={}",
        files.len(),
        chunks.len(),
        plan.len()
    );
    println!(
        "provider={:?} task={} concurrency={} cap_calls={} cap_tokens={}",
        config.provider,
        task_name(config.task),
        config.concurrency,
        cap_word(config.max_calls),
        cap_word(config.max_tokens)
    );
    println!("token estimate={est_tokens} (chars/4, input side only)");
    if config.provider == Provider::Both && config.both == BothMode::Race {
        println!("warning: race mode spends two requests per chunk");
    }
    if config.max_calls > 0 && plan.len() as u64 > config.max_calls {
        println!("warning: call cap stops this run early");
    }
    if config.max_tokens > 0 && est_tokens > config.max_tokens {
        println!("warning: token cap stops this run early");
    }
    println!("cost: see provider pricing for your model");
}

/// Build live clients. Fails closed without creds. Mock needs none.
fn build_clients(
    config: &ai_batch::Config,
) -> Result<(Option<ProviderClient>, Option<ProviderClient>), String> {
    let need_baseten = matches!(config.provider, Provider::Baseten | Provider::Both);
    let need_modal = matches!(config.provider, Provider::Modal | Provider::Both);
    let baseten = if need_baseten {
        let key = config
            .baseten_key
            .as_ref()
            .filter(|k| !k.is_empty())
            .ok_or("Baseten needs BASETEN_API_KEY")?;
        let model = config
            .baseten_model
            .as_ref()
            .filter(|m| !m.is_empty())
            .ok_or("Baseten needs BASETEN_MODEL")?;
        Some(ProviderClient::baseten(&config.baseten_base, key, model)?)
    } else {
        None
    };
    let modal = if need_modal {
        let id = config
            .modal_id
            .as_ref()
            .filter(|k| !k.is_empty())
            .ok_or("Modal needs MODAL_PROXY_TOKEN_ID plus MODAL_PROXY_TOKEN_SECRET")?;
        let url = config
            .modal_url
            .as_ref()
            .filter(|u| !u.is_empty())
            .ok_or("Modal needs MODAL_MODEL_URL")?;
        let model = config
            .modal_model
            .as_ref()
            .filter(|m| !m.is_empty())
            .ok_or("Modal needs MODAL_MODEL")?;
        Some(ProviderClient::modal(url, id, model)?)
    } else {
        None
    };
    Ok((baseten, modal))
}
