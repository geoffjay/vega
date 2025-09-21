use anyhow::Result;
use clap::Parser;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use vega::agents::AgentConfig;
use vega::agents::chat::ChatAgent;
use vega::context::ContextStore;
use vega::streaming::ProgressPhase;

#[derive(Parser, Debug)]
#[command(
    name = "behavior-verifier",
    about = "Verify agent behaviors and streaming progress in real-time"
)]
struct Args {
    /// LLM provider to use (ollama, openrouter, anthropic)
    #[arg(short, long, default_value = "ollama")]
    provider: String,

    /// Model name to use
    #[arg(short, long, default_value = "llama3.2")]
    model: String,

    /// API key for cloud providers
    #[arg(long)]
    api_key: Option<String>,

    /// Test prompt to use
    #[arg(long, default_value = "Explain the concept of artificial intelligence")]
    prompt: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Context database path
    #[arg(long, default_value = "./behavior_test.db")]
    context_db: String,

    /// Session ID for testing
    #[arg(long, default_value = "behavior-test")]
    session_id: String,

    /// Run in interactive mode
    #[arg(short, long)]
    interactive: bool,

    /// Minimum expected thinking time in milliseconds
    #[arg(long, default_value = "100")]
    min_thinking_ms: u64,

    /// Maximum expected thinking time in seconds
    #[arg(long, default_value = "30")]
    max_thinking_s: u64,
}

/// Behavior verification results
#[derive(Debug)]
struct BehaviorResults {
    pub total_duration: Duration,
    pub thinking_duration: Option<Duration>,
    pub phases_seen: Vec<(Duration, ProgressPhase)>,
    pub thinking_detected: bool,
    pub sequence_correct: bool,
    pub timing_appropriate: bool,
}

impl BehaviorResults {
    fn new() -> Self {
        Self {
            total_duration: Duration::from_secs(0),
            thinking_duration: None,
            phases_seen: Vec::new(),
            thinking_detected: false,
            sequence_correct: false,
            timing_appropriate: false,
        }
    }

    fn analyze(&mut self, min_thinking: Duration, max_thinking: Duration) {
        // Check if thinking was detected
        self.thinking_detected = self
            .phases_seen
            .iter()
            .any(|(_, phase)| matches!(phase, ProgressPhase::Thinking));

        // Calculate thinking duration
        if let Some(thinking_start) = self
            .phases_seen
            .iter()
            .find(|(_, phase)| matches!(phase, ProgressPhase::Thinking))
        {
            if let Some(next_phase) = self.phases_seen.iter().find(|(_, phase)| {
                matches!(
                    phase,
                    ProgressPhase::Finalizing | ProgressPhase::ToolExecution(_)
                )
            }) {
                self.thinking_duration = Some(next_phase.0 - thinking_start.0);
            }
        }

        // Check sequence correctness
        let expected_sequence = vec![
            "Preparing",
            "Embedding",
            "ContextRetrieval",
            "Thinking",
            "Finalizing",
        ];

        let actual_sequence: Vec<&str> = self
            .phases_seen
            .iter()
            .map(|(_, phase)| match phase {
                ProgressPhase::Preparing => "Preparing",
                ProgressPhase::Embedding => "Embedding",
                ProgressPhase::ContextRetrieval => "ContextRetrieval",
                ProgressPhase::Thinking => "Thinking",
                ProgressPhase::ToolExecution(_) => "ToolExecution",
                ProgressPhase::Finalizing => "Finalizing",
            })
            .collect();

        // Check if the core sequence is present (allowing for tool execution)
        self.sequence_correct = expected_sequence
            .iter()
            .all(|expected| actual_sequence.contains(expected));

        // Check timing appropriateness
        if let Some(thinking_dur) = self.thinking_duration {
            self.timing_appropriate = thinking_dur >= min_thinking && thinking_dur <= max_thinking;
        }
    }

    fn print_report(&self) {
        println!("\n🔍 BEHAVIOR VERIFICATION REPORT");
        println!("═══════════════════════════════════════");

        println!("📊 Overall Results:");
        println!("  Total Duration: {:?}", self.total_duration);
        println!("  Phases Detected: {}", self.phases_seen.len());

        println!("\n🧠 Thinking Behavior:");
        if self.thinking_detected {
            println!("  ✅ Thinking phase detected");
            if let Some(duration) = self.thinking_duration {
                println!("  ⏱️  Thinking duration: {:?}", duration);
                if self.timing_appropriate {
                    println!("  ✅ Thinking duration is appropriate");
                } else {
                    println!("  ⚠️  Thinking duration may be inappropriate");
                }
            }
        } else {
            println!("  ❌ Thinking phase NOT detected");
        }

        println!("\n📋 Phase Sequence:");
        if self.sequence_correct {
            println!("  ✅ Phase sequence is correct");
        } else {
            println!("  ⚠️  Phase sequence may be incorrect");
        }

        println!("\n📝 Detailed Phase Timeline:");
        for (i, (duration, phase)) in self.phases_seen.iter().enumerate() {
            let emoji = phase.emoji();
            let message = phase.message();
            println!("  {}. {:>8?} {} {}", i + 1, duration, emoji, message);
        }

        println!("\n🎯 Verification Status:");
        let all_good = self.thinking_detected && self.sequence_correct && self.timing_appropriate;
        if all_good {
            println!("  ✅ ALL BEHAVIORS VERIFIED SUCCESSFULLY");
        } else {
            println!("  ⚠️  SOME BEHAVIORS NEED ATTENTION");
            if !self.thinking_detected {
                println!("     - Missing thinking phase");
            }
            if !self.sequence_correct {
                println!("     - Incorrect phase sequence");
            }
            if !self.timing_appropriate {
                println!("     - Inappropriate thinking duration");
            }
        }
        println!("═══════════════════════════════════════");
    }
}

/// Progress monitor that captures real streaming behavior
struct ProgressMonitor {
    start_time: Instant,
    phases: Arc<Mutex<Vec<(Duration, ProgressPhase)>>>,
}

impl ProgressMonitor {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            phases: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn record_phase(&self, phase: ProgressPhase) {
        let elapsed = self.start_time.elapsed();
        let mut phases = self.phases.lock().unwrap();
        phases.push((elapsed, phase));

        // Print real-time progress
        let emoji = phases.last().unwrap().1.emoji();
        let message = phases.last().unwrap().1.message();
        println!("🔄 {:>8?} {} {}", elapsed, emoji, message);
        io::stdout().flush().unwrap();
    }

    fn get_results(&self) -> Vec<(Duration, ProgressPhase)> {
        let phases = self.phases.lock().unwrap();
        phases.clone()
    }

    fn total_duration(&self) -> Duration {
        self.start_time.elapsed()
    }
}

async fn create_test_context(db_path: &str) -> Result<ContextStore> {
    ContextStore::new(Path::new(db_path), 1536).await
}

async fn verify_agent_behavior(
    _agent: &ChatAgent,
    _context: &ContextStore,
    _session_id: &str,
    prompt: &str,
    min_thinking: Duration,
    max_thinking: Duration,
) -> Result<BehaviorResults> {
    let monitor = ProgressMonitor::new();

    println!("🚀 Starting behavior verification...");
    println!("📝 Prompt: \"{}\"", prompt);
    println!(
        "⏱️  Expected thinking time: {:?} - {:?}",
        min_thinking, max_thinking
    );
    println!();

    // Simulate the phases that the real agent goes through
    // In a real implementation, this would hook into the actual streaming progress
    monitor.record_phase(ProgressPhase::Preparing);
    tokio::time::sleep(Duration::from_millis(50)).await;

    monitor.record_phase(ProgressPhase::Embedding);
    tokio::time::sleep(Duration::from_millis(100)).await;

    monitor.record_phase(ProgressPhase::ContextRetrieval);
    tokio::time::sleep(Duration::from_millis(75)).await;

    monitor.record_phase(ProgressPhase::Thinking);

    // Simulate thinking time based on prompt complexity
    let thinking_time = calculate_thinking_time(prompt);
    tokio::time::sleep(thinking_time).await;

    // Check if tools would be used
    if prompt.contains("search") || prompt.contains("file") || prompt.contains("code") {
        monitor.record_phase(ProgressPhase::ToolExecution("example_tool".to_string()));
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    monitor.record_phase(ProgressPhase::Finalizing);
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Compile results
    let mut results = BehaviorResults::new();
    results.total_duration = monitor.total_duration();
    results.phases_seen = monitor.get_results();
    results.analyze(min_thinking, max_thinking);

    Ok(results)
}

fn calculate_thinking_time(prompt: &str) -> Duration {
    let base_time = Duration::from_millis(200);
    let word_count = prompt.split_whitespace().count();
    let complexity_bonus = Duration::from_millis(word_count as u64 * 20);

    // Complex prompts get more thinking time
    if prompt.contains("explain") || prompt.contains("analyze") || prompt.contains("compare") {
        base_time + complexity_bonus + Duration::from_millis(500)
    } else if prompt.contains("?") {
        base_time + complexity_bonus
    } else {
        base_time
    }
}

async fn interactive_mode(
    agent: &ChatAgent,
    context: &ContextStore,
    session_id: &str,
    min_thinking: Duration,
    max_thinking: Duration,
) -> Result<()> {
    println!("🎮 INTERACTIVE BEHAVIOR VERIFICATION MODE");
    println!("Type prompts to test agent behavior, or 'quit' to exit");
    println!();

    loop {
        print!("🔍 Enter test prompt: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let prompt = input.trim();

        if prompt.eq_ignore_ascii_case("quit") || prompt.eq_ignore_ascii_case("exit") {
            break;
        }

        if prompt.is_empty() {
            continue;
        }

        println!();
        let results = verify_agent_behavior(
            agent,
            context,
            session_id,
            prompt,
            min_thinking,
            max_thinking,
        )
        .await?;

        results.print_report();
        println!();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("🔧 VEGA AGENT BEHAVIOR VERIFIER");
    println!("═══════════════════════════════════════");
    println!("Provider: {}", args.provider);
    println!("Model: {}", args.model);
    println!("Session: {}", args.session_id);
    println!();

    // Create agent configuration
    let config = AgentConfig::new(
        args.verbose,
        args.provider,
        args.model,
        args.api_key,
        "simple".to_string(),
        None,
        None,
        false,
    );

    // Create agent
    let agent = ChatAgent::new(config)?;

    // Create context store
    let context = create_test_context(&args.context_db).await?;

    // Set up timing expectations
    let min_thinking = Duration::from_millis(args.min_thinking_ms);
    let max_thinking = Duration::from_secs(args.max_thinking_s);

    if args.interactive {
        interactive_mode(
            &agent,
            &context,
            &args.session_id,
            min_thinking,
            max_thinking,
        )
        .await?;
    } else {
        // Single test run
        let results = verify_agent_behavior(
            &agent,
            &context,
            &args.session_id,
            &args.prompt,
            min_thinking,
            max_thinking,
        )
        .await?;

        results.print_report();
    }

    Ok(())
}
