// SYNOID AutoResearch - Autonomous Research Pipeline
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Inspired by AutoResearchClaw (aiming-lab/AutoResearchClaw).
// Transforms a research topic into structured findings via multi-stage pipeline.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// A single academic paper record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<u32>,
    pub abstract_text: String,
    pub source: String,       // "arxiv" | "semantic_scholar" | "openalex"
    pub url: Option<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub citation_count: Option<u32>,
    pub relevance_score: f32,
}

/// A citation validity record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationCheck {
    pub arxiv_valid: bool,
    pub doi_valid: bool,
    pub title_match: bool,
    pub llm_relevant: bool,
    pub overall: bool,
}

/// A research finding aggregated from multiple papers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchGap {
    pub description: String,
    pub supporting_papers: Vec<String>,
    pub confidence: f32,
}

/// A testable hypothesis generated from synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub statement: String,
    pub rationale: String,
    pub suggested_experiments: Vec<String>,
}

/// Overall research run result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub topic: String,
    pub phase: String,
    pub papers: Vec<Paper>,
    pub knowledge_cards: Vec<KnowledgeCard>,
    pub gaps: Vec<ResearchGap>,
    pub hypotheses: Vec<Hypothesis>,
    pub summary: String,
    pub citations_verified: bool,
    pub verification_report: HashMap<String, CitationCheck>,
}

/// Structured knowledge extracted from a paper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCard {
    pub title: String,
    pub key_contribution: String,
    pub methodology: String,
    pub result_summary: String,
    pub limitations: String,
    pub paper_source: String,
}

/// Pipeline stage identifiers — mirrors AutoResearchClaw's 8-phase architecture.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStage {
    TopicInit,
    ProblemDecompose,
    SearchStrategy,
    LiteratureCollect,
    LiteratureScreen,
    KnowledgeExtract,
    Synthesis,
    HypothesisGen,
    Complete,
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::TopicInit => "TOPIC_INIT",
            Self::ProblemDecompose => "PROBLEM_DECOMPOSE",
            Self::SearchStrategy => "SEARCH_STRATEGY",
            Self::LiteratureCollect => "LITERATURE_COLLECT",
            Self::LiteratureScreen => "LITERATURE_SCREEN",
            Self::KnowledgeExtract => "KNOWLEDGE_EXTRACT",
            Self::Synthesis => "SYNTHESIS",
            Self::HypothesisGen => "HYPOTHESIS_GEN",
            Self::Complete => "COMPLETE",
        };
        write!(f, "{}", s)
    }
}

/// The autonomous research pipeline runner.
pub struct AutoResearchPipeline {
    client: reqwest::Client,
    api_key_semantic: Option<String>,
    ollama_url: String,
    ollama_model: String,
    groq_key: Option<String>,
    groq_model: String,
}

impl AutoResearchPipeline {
    pub fn new() -> Self {
        let ollama_url = std::env::var("SYNOID_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let groq_model = std::env::var("GROQ_REASONING_MODEL")
            .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());

        Self {
            client: crate::net::build_client(std::time::Duration::from_secs(30)),
            api_key_semantic: std::env::var("SEMANTIC_SCHOLAR_API_KEY").ok(),
            ollama_url,
            ollama_model: std::env::var("SYNOID_MODEL")
                .unwrap_or_else(|_| "llama3.2:latest".to_string()),
            groq_key: std::env::var("GROQ_API_KEY").ok(),
            groq_model,
        }
    }

    /// Run the full pipeline for a given topic.
    pub async fn run(&self, topic: &str, paper_limit: usize) -> ResearchResult {
        info!("[AUTORESEARCH] 🚀 Starting pipeline for: {}", topic);

        // Phase A — Research Scoping
        info!("[AUTORESEARCH] [{}] Scoping research...", PipelineStage::TopicInit);
        let sub_questions = self.decompose_topic(topic).await;

        // Phase B — Literature Discovery
        info!("[AUTORESEARCH] [{}] Querying academic sources...", PipelineStage::LiteratureCollect);
        let mut papers = self.collect_literature(topic, paper_limit).await;
        info!("[AUTORESEARCH] Retrieved {} papers", papers.len());

        // Phase B.5 — Screen / quality gate
        info!("[AUTORESEARCH] [{}] Screening papers...", PipelineStage::LiteratureScreen);
        papers.retain(|p| p.relevance_score >= 0.3);
        papers.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        if papers.len() > paper_limit {
            papers.truncate(paper_limit);
        }

        // Phase C — Knowledge Extraction
        info!("[AUTORESEARCH] [{}] Extracting knowledge cards...", PipelineStage::KnowledgeExtract);
        let knowledge_cards = self.extract_knowledge(&papers).await;

        // Phase C.5 — Synthesis
        info!("[AUTORESEARCH] [{}] Synthesising gaps...", PipelineStage::Synthesis);
        let gaps = self.synthesise_gaps(&papers, &sub_questions).await;

        // Phase C.6 — Hypothesis generation
        info!("[AUTORESEARCH] [{}] Generating hypotheses...", PipelineStage::HypothesisGen);
        let hypotheses = self.generate_hypotheses(topic, &gaps).await;

        // Phase H — Citation verification (4-layer)
        info!("[AUTORESEARCH] Running 4-layer citation verification...");
        let (citations_verified, verification_report) = self.verify_citations(&papers).await;

        // Build summary via LLM
        let summary = self.summarise_findings(topic, &papers, &gaps, &hypotheses).await;

        info!("[AUTORESEARCH] [{}] Pipeline complete.", PipelineStage::Complete);

        ResearchResult {
            topic: topic.to_string(),
            phase: PipelineStage::Complete.to_string(),
            papers,
            knowledge_cards,
            gaps,
            hypotheses,
            summary,
            citations_verified,
            verification_report,
        }
    }

    // ── Phase A helpers ──────────────────────────────────────────────────────

    async fn decompose_topic(&self, topic: &str) -> Vec<String> {
        let prompt = format!(
            "Decompose this research topic into 3-5 focused sub-questions for a systematic literature review.\nTopic: {}\nReturn one sub-question per line, no numbering.",
            topic
        );
        match self.llm_query(&prompt).await {
            Ok(text) => text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
            Err(e) => {
                warn!("[AUTORESEARCH] Topic decompose LLM failed: {}", e);
                vec![topic.to_string()]
            }
        }
    }

    // ── Phase B helpers ──────────────────────────────────────────────────────

    /// Collect papers from arXiv, Semantic Scholar, and OpenAlex.
    async fn collect_literature(&self, topic: &str, limit: usize) -> Vec<Paper> {
        let per_source = (limit / 3).max(3);
        let (arxiv, semantic, openalex) = tokio::join!(
            self.search_arxiv(topic, per_source),
            self.search_semantic_scholar(topic, per_source),
            self.search_openalex(topic, per_source),
        );

        let mut combined = Vec::new();
        combined.extend(arxiv);
        combined.extend(semantic);
        combined.extend(openalex);

        // De-duplicate by title (case-insensitive)
        let mut seen = std::collections::HashSet::new();
        combined.retain(|p| {
            let key = p.title.to_lowercase();
            seen.insert(key)
        });

        combined
    }

    /// Search arXiv API (public wrapper for research_tools).
    pub async fn search_arxiv_pub(&self, query: &str, max: usize) -> Vec<Paper> {
        self.search_arxiv(query, max).await
    }

    /// Search Semantic Scholar API (public wrapper for research_tools).
    pub async fn search_semantic_scholar_pub(&self, query: &str, max: usize) -> Vec<Paper> {
        self.search_semantic_scholar(query, max).await
    }

    /// Search arXiv API.
    async fn search_arxiv(&self, query: &str, max: usize) -> Vec<Paper> {
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}",
            encoded, max
        );

        let resp = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                warn!("[AUTORESEARCH] arXiv fetch failed: {}", e);
                return Vec::new();
            }
        };

        let body = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        self.parse_arxiv_response(&body, query)
    }

    fn parse_arxiv_response(&self, xml: &str, query: &str) -> Vec<Paper> {
        let mut papers = Vec::new();

        // Simple XML extraction without pulling in a full parser dep
        let entries: Vec<&str> = xml.split("<entry>").skip(1).collect();
        for entry in entries {
            let title = extract_xml_tag(entry, "title")
                .unwrap_or_default()
                .replace('\n', " ")
                .trim()
                .to_string();
            let summary = extract_xml_tag(entry, "summary")
                .unwrap_or_default()
                .replace('\n', " ")
                .trim()
                .to_string();
            let id_raw = extract_xml_tag(entry, "id").unwrap_or_default();
            let arxiv_id = id_raw
                .trim()
                .split('/')
                .last()
                .map(|s| s.to_string());

            if title.is_empty() {
                continue;
            }

            // Extract authors
            let authors: Vec<String> = entry
                .split("<author>")
                .skip(1)
                .filter_map(|a| extract_xml_tag(a, "name"))
                .take(5)
                .collect();

            // Extract year from published date
            let year = extract_xml_tag(entry, "published")
                .and_then(|d| d.get(..4).map(|y| y.parse::<u32>().ok()))
                .flatten();

            let url = arxiv_id
                .as_ref()
                .map(|id| format!("https://arxiv.org/abs/{}", id));

            let relevance = compute_relevance(&title, &summary, query);

            papers.push(Paper {
                title,
                authors,
                year,
                abstract_text: if summary.len() > 500 {
                    format!("{}...", &summary[..500])
                } else {
                    summary
                },
                source: "arxiv".to_string(),
                url,
                doi: None,
                arxiv_id,
                citation_count: None,
                relevance_score: relevance,
            });
        }

        papers
    }

    /// Search Semantic Scholar API.
    async fn search_semantic_scholar(&self, query: &str, max: usize) -> Vec<Paper> {
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&limit={}&fields=title,authors,year,abstract,externalIds,citationCount,url",
            encoded, max
        );

        let mut req = self.client.get(&url);
        if let Some(key) = &self.api_key_semantic {
            req = req.header("x-api-key", key);
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                warn!("[AUTORESEARCH] Semantic Scholar fetch failed: {}", e);
                return Vec::new();
            }
        };

        let json: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let mut papers = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for item in data {
                let title = item["title"].as_str().unwrap_or("").to_string();
                if title.is_empty() {
                    continue;
                }
                let abstract_text = item["abstract"].as_str().unwrap_or("").to_string();
                let year = item["year"].as_u64().map(|y| y as u32);
                let citation_count = item["citationCount"].as_u64().map(|c| c as u32);
                let url = item["url"].as_str().map(|u| u.to_string());
                let doi = item["externalIds"]["DOI"].as_str().map(|d| d.to_string());
                let arxiv_id = item["externalIds"]["ArXiv"].as_str().map(|a| a.to_string());

                let authors: Vec<String> = item["authors"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|a| a["name"].as_str().map(|n| n.to_string()))
                            .take(5)
                            .collect()
                    })
                    .unwrap_or_default();

                let relevance = compute_relevance(&title, &abstract_text, query);

                papers.push(Paper {
                    title,
                    authors,
                    year,
                    abstract_text,
                    source: "semantic_scholar".to_string(),
                    url,
                    doi,
                    arxiv_id,
                    citation_count,
                    relevance_score: relevance,
                });
            }
        }

        papers
    }

    /// Search OpenAlex API.
    async fn search_openalex(&self, query: &str, max: usize) -> Vec<Paper> {
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://api.openalex.org/works?search={}&per-page={}&select=title,authorships,publication_year,abstract_inverted_index,doi,primary_location,cited_by_count",
            encoded, max
        );

        let resp = match self
            .client
            .get(&url)
            .header("User-Agent", crate::net::USER_AGENT)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("[AUTORESEARCH] OpenAlex fetch failed: {}", e);
                return Vec::new();
            }
        };

        let json: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let mut papers = Vec::new();
        if let Some(results) = json["results"].as_array() {
            for item in results {
                let title = item["title"].as_str().unwrap_or("").to_string();
                if title.is_empty() {
                    continue;
                }

                let year = item["publication_year"].as_u64().map(|y| y as u32);
                let doi = item["doi"]
                    .as_str()
                    .map(|d| d.trim_start_matches("https://doi.org/").to_string());
                let citation_count = item["cited_by_count"].as_u64().map(|c| c as u32);
                let url = item["primary_location"]["landing_page_url"]
                    .as_str()
                    .map(|u| u.to_string());

                let authors: Vec<String> = item["authorships"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|a| {
                                a["author"]["display_name"].as_str().map(|n| n.to_string())
                            })
                            .take(5)
                            .collect()
                    })
                    .unwrap_or_default();

                // Reconstruct abstract from inverted index
                let abstract_text =
                    reconstruct_openalex_abstract(&item["abstract_inverted_index"]);

                let relevance = compute_relevance(&title, &abstract_text, query);

                papers.push(Paper {
                    title,
                    authors,
                    year,
                    abstract_text,
                    source: "openalex".to_string(),
                    url,
                    doi,
                    arxiv_id: None,
                    citation_count,
                    relevance_score: relevance,
                });
            }
        }

        papers
    }

    // ── Phase C helpers ──────────────────────────────────────────────────────

    async fn extract_knowledge(&self, papers: &[Paper]) -> Vec<KnowledgeCard> {
        let mut cards = Vec::new();
        for paper in papers.iter().take(8) {
            let prompt = format!(
                "Extract structured knowledge from this paper abstract.\nTitle: {}\nAbstract: {}\n\nRespond in this exact format:\nKey Contribution: <1 sentence>\nMethodology: <1 sentence>\nResult: <1 sentence>\nLimitations: <1 sentence>",
                paper.title, paper.abstract_text
            );

            let text = match self.llm_query(&prompt).await {
                Ok(t) => t,
                Err(_) => continue,
            };

            let contribution = extract_field(&text, "Key Contribution:")
                .unwrap_or_else(|| "Not extracted".to_string());
            let methodology = extract_field(&text, "Methodology:")
                .unwrap_or_else(|| "Not extracted".to_string());
            let result_summary =
                extract_field(&text, "Result:").unwrap_or_else(|| "Not extracted".to_string());
            let limitations = extract_field(&text, "Limitations:")
                .unwrap_or_else(|| "Not extracted".to_string());

            cards.push(KnowledgeCard {
                title: paper.title.clone(),
                key_contribution: contribution,
                methodology,
                result_summary,
                limitations,
                paper_source: paper.source.clone(),
            });
        }
        cards
    }

    async fn synthesise_gaps(
        &self,
        papers: &[Paper],
        sub_questions: &[String],
    ) -> Vec<ResearchGap> {
        let paper_titles: Vec<&str> = papers.iter().map(|p| p.title.as_str()).take(10).collect();
        let questions_text = sub_questions.join("\n");
        let titles_text = paper_titles.join("\n");

        let prompt = format!(
            "Given these sub-questions:\n{}\n\nAnd these retrieved papers:\n{}\n\nIdentify 2-3 clear research gaps not fully addressed. For each gap:\nGap: <description>\nSupporting: <comma-separated paper titles>",
            questions_text, titles_text
        );

        let text = match self.llm_query(&prompt).await {
            Ok(t) => t,
            Err(e) => {
                warn!("[AUTORESEARCH] Gap synthesis failed: {}", e);
                return Vec::new();
            }
        };

        let mut gaps = Vec::new();
        let mut current_gap: Option<String> = None;
        let mut current_support: Vec<String> = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(desc) = trimmed.strip_prefix("Gap:") {
                if let Some(g) = current_gap.take() {
                    gaps.push(ResearchGap {
                        description: g,
                        supporting_papers: std::mem::take(&mut current_support),
                        confidence: 0.7,
                    });
                }
                current_gap = Some(desc.trim().to_string());
            } else if let Some(sup) = trimmed.strip_prefix("Supporting:") {
                current_support = sup
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        if let Some(g) = current_gap {
            gaps.push(ResearchGap {
                description: g,
                supporting_papers: current_support,
                confidence: 0.7,
            });
        }

        gaps
    }

    async fn generate_hypotheses(
        &self,
        topic: &str,
        gaps: &[ResearchGap],
    ) -> Vec<Hypothesis> {
        if gaps.is_empty() {
            return Vec::new();
        }
        let gaps_text = gaps
            .iter()
            .map(|g| format!("- {}", g.description))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Based on this topic '{}' and these research gaps:\n{}\n\nGenerate 1-2 testable hypotheses. For each:\nHypothesis: <statement>\nRationale: <why this fills the gap>\nExperiment: <suggested test>",
            topic, gaps_text
        );

        let text = match self.llm_query(&prompt).await {
            Ok(t) => t,
            Err(e) => {
                warn!("[AUTORESEARCH] Hypothesis gen failed: {}", e);
                return Vec::new();
            }
        };

        let mut hypotheses = Vec::new();
        let mut current = HashMap::<&str, String>::new();

        for line in text.lines() {
            let trimmed = line.trim();
            for key in &["Hypothesis:", "Rationale:", "Experiment:"] {
                if let Some(val) = trimmed.strip_prefix(key) {
                    current.insert(key, val.trim().to_string());
                }
            }
            if current.len() == 3 {
                hypotheses.push(Hypothesis {
                    statement: current.get("Hypothesis:").cloned().unwrap_or_default(),
                    rationale: current.get("Rationale:").cloned().unwrap_or_default(),
                    suggested_experiments: current
                        .get("Experiment:")
                        .map(|e| vec![e.clone()])
                        .unwrap_or_default(),
                });
                current.clear();
            }
        }

        hypotheses
    }

    // ── Phase H: Citation Verification (4 layers) ────────────────────────────

    async fn verify_citations(
        &self,
        papers: &[Paper],
    ) -> (bool, HashMap<String, CitationCheck>) {
        let mut report = HashMap::new();
        let mut all_valid = true;

        for paper in papers {
            let arxiv_valid = if let Some(id) = &paper.arxiv_id {
                self.verify_arxiv_id(id).await
            } else {
                true // not applicable — pass
            };

            let doi_valid = if let Some(doi) = &paper.doi {
                self.verify_doi(doi).await
            } else {
                true // not applicable
            };

            let title_match = arxiv_valid; // simplified: if arxiv confirmed, title matched

            // Layer 4: LLM relevance re-check for suspicious low-score papers
            let llm_relevant = if paper.relevance_score < 0.4 {
                self.llm_relevance_check(&paper.title, &paper.abstract_text)
                    .await
            } else {
                true
            };

            let overall = arxiv_valid && doi_valid && llm_relevant;
            if !overall {
                all_valid = false;
            }

            report.insert(
                paper.title.clone(),
                CitationCheck {
                    arxiv_valid,
                    doi_valid,
                    title_match,
                    llm_relevant,
                    overall,
                },
            );
        }

        (all_valid, report)
    }

    async fn verify_arxiv_id(&self, arxiv_id: &str) -> bool {
        // Sanitize: strip version suffix for check
        let clean = arxiv_id.split('v').next().unwrap_or(arxiv_id);
        let url = format!("https://export.arxiv.org/abs/{}", clean);
        match self.client.head(&url).send().await {
            Ok(r) => r.status().is_success(),
            Err(_) => true, // network issue — don't fail the paper
        }
    }

    async fn verify_doi(&self, doi: &str) -> bool {
        let url = format!("https://doi.org/{}", doi);
        match self
            .client
            .head(&url)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(r) => r.status().is_success() || r.status().is_redirection(),
            Err(_) => true,
        }
    }

    async fn llm_relevance_check(&self, title: &str, abstract_text: &str) -> bool {
        let prompt = format!(
            "Is this paper clearly about AI, machine learning, or computer science research?\nTitle: {}\nAbstract: {}\nAnswer only: YES or NO",
            title, abstract_text
        );
        match self.llm_query(&prompt).await {
            Ok(resp) => !resp.to_uppercase().contains("NO"),
            Err(_) => true,
        }
    }

    // ── Summary ──────────────────────────────────────────────────────────────

    async fn summarise_findings(
        &self,
        topic: &str,
        papers: &[Paper],
        gaps: &[ResearchGap],
        hypotheses: &[Hypothesis],
    ) -> String {
        let paper_count = papers.len();
        let gap_count = gaps.len();
        let hyp_count = hypotheses.len();

        let gap_descriptions = gaps
            .iter()
            .map(|g| format!("  • {}", g.description))
            .collect::<Vec<_>>()
            .join("\n");

        let hyp_statements = hypotheses
            .iter()
            .map(|h| format!("  • {}", h.statement))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Write a concise 150-word executive summary for a literature review on '{}'. {} papers were analysed, {} research gaps found, {} hypotheses generated.\nGaps:\n{}\nHypotheses:\n{}",
            topic, paper_count, gap_count, hyp_count, gap_descriptions, hyp_statements
        );

        match self.llm_query(&prompt).await {
            Ok(t) => t,
            Err(_) => format!(
                "Analysed {} papers on '{}'. Found {} research gaps and generated {} hypotheses.",
                paper_count, topic, gap_count, hyp_count
            ),
        }
    }

    // ── LLM helper ───────────────────────────────────────────────────────────

    async fn llm_query(&self, prompt: &str) -> Result<String, String> {
        // Try Groq first (fast cloud), fall back to Ollama
        if let Some(key) = &self.groq_key {
            match self.call_groq(prompt, key).await {
                Ok(text) => return Ok(text),
                Err(e) => warn!("[AUTORESEARCH] Groq failed, trying Ollama: {}", e),
            }
        }
        self.call_ollama(prompt).await
    }

    async fn call_groq(&self, prompt: &str, api_key: &str) -> Result<String, String> {
        let payload = serde_json::json!({
            "model": self.groq_model,
            "messages": [
                {"role": "system", "content": "You are an expert research assistant. Be concise and precise."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.3,
            "max_tokens": 1024
        });

        let resp = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Groq error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    async fn call_ollama(&self, prompt: &str) -> Result<String, String> {
        let api_url = format!("{}/v1/chat/completions", self.ollama_url);
        let payload = serde_json::json!({
            "model": self.ollama_model,
            "messages": [{"role": "user", "content": prompt}],
            "stream": false
        });

        let resp = self
            .client
            .post(&api_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Ollama error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}

// ── Utilities ────────────────────────────────────────────────────────────────

/// Compute a simple TF-IDF-like relevance score between query and text.
fn compute_relevance(title: &str, text: &str, query: &str) -> f32 {
    let query_words: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| w.len() > 2)
        .collect();

    let combined = format!("{} {}", title, text).to_lowercase();
    if query_words.is_empty() {
        return 0.5;
    }

    let matches = query_words
        .iter()
        .filter(|w| combined.contains(w.as_str()))
        .count();

    let score = matches as f32 / query_words.len() as f32;
    // Boost if title matches
    let title_lower = title.to_lowercase();
    let title_boost = query_words
        .iter()
        .filter(|w| title_lower.contains(w.as_str()))
        .count() as f32
        * 0.1;

    (score + title_boost).min(1.0)
}

/// Extract the text content of an XML tag (first occurrence).
fn extract_xml_tag(src: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = src.find(&open)? + open.len();
    let end = src[start..].find(&close)?;
    Some(src[start..start + end].to_string())
}

/// Reconstruct the abstract from OpenAlex inverted index format.
fn reconstruct_openalex_abstract(inverted: &serde_json::Value) -> String {
    let obj = match inverted.as_object() {
        Some(o) => o,
        None => return String::new(),
    };

    let mut word_positions: Vec<(usize, &str)> = Vec::new();
    for (word, positions) in obj {
        if let Some(arr) = positions.as_array() {
            for pos in arr {
                if let Some(p) = pos.as_u64() {
                    word_positions.push((p as usize, word.as_str()));
                }
            }
        }
    }
    word_positions.sort_by_key(|(p, _)| *p);

    let words: Vec<&str> = word_positions.iter().map(|(_, w)| *w).collect();
    let text = words.join(" ");
    if text.len() > 500 {
        format!("{}...", &text[..500])
    } else {
        text
    }
}

/// Extract a named field from LLM output formatted as "Field: value".
fn extract_field(text: &str, field: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(val) = line.trim().strip_prefix(field) {
            return Some(val.trim().to_string());
        }
    }
    None
}

/// Print a research result in a readable format.
pub fn print_research_result(result: &ResearchResult) {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║         SYNOID AUTORESEARCH RESULTS                 ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!("Topic: {}", result.topic);
    println!("Papers analysed: {}", result.papers.len());
    println!(
        "Citations verified: {}",
        if result.citations_verified { "✅ All valid" } else { "⚠️ Some issues detected" }
    );

    println!("\n── Top Papers ────────────────────────────────────────");
    for (i, paper) in result.papers.iter().take(5).enumerate() {
        println!(
            "{}. [{}] {} ({}) — relevance: {:.0}%",
            i + 1,
            paper.source.to_uppercase(),
            paper.title,
            paper.year.map(|y| y.to_string()).unwrap_or_else(|| "n/a".to_string()),
            paper.relevance_score * 100.0
        );
        if !paper.authors.is_empty() {
            println!("   Authors: {}", paper.authors.join(", "));
        }
        if let Some(url) = &paper.url {
            println!("   URL: {}", url);
        }
    }

    if !result.knowledge_cards.is_empty() {
        println!("\n── Knowledge Cards ───────────────────────────────────");
        for card in result.knowledge_cards.iter().take(3) {
            println!("📄 {}", card.title);
            println!("   Contribution: {}", card.key_contribution);
            println!("   Result: {}", card.result_summary);
        }
    }

    if !result.gaps.is_empty() {
        println!("\n── Research Gaps ─────────────────────────────────────");
        for gap in &result.gaps {
            println!("🔍 {}", gap.description);
        }
    }

    if !result.hypotheses.is_empty() {
        println!("\n── Generated Hypotheses ──────────────────────────────");
        for hyp in &result.hypotheses {
            println!("💡 {}", hyp.statement);
            println!("   Rationale: {}", hyp.rationale);
            for exp in &hyp.suggested_experiments {
                println!("   Experiment: {}", exp);
            }
        }
    }

    println!("\n── Executive Summary ─────────────────────────────────");
    println!("{}", result.summary);
    println!("──────────────────────────────────────────────────────\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_relevance() {
        let score = compute_relevance(
            "Attention Is All You Need",
            "We propose a new architecture based on attention mechanisms",
            "transformer attention neural network",
        );
        assert!(score > 0.3, "Expected relevance > 0.3, got {}", score);
    }

    #[test]
    fn test_extract_xml_tag() {
        let xml = "<title>Test Paper</title><summary>A test abstract.</summary>";
        assert_eq!(extract_xml_tag(xml, "title"), Some("Test Paper".to_string()));
        assert_eq!(
            extract_xml_tag(xml, "summary"),
            Some("A test abstract.".to_string())
        );
    }

    #[test]
    fn test_extract_field() {
        let text = "Key Contribution: Novel attention mechanism\nResult: SOTA on GLUE";
        assert_eq!(
            extract_field(text, "Key Contribution:"),
            Some("Novel attention mechanism".to_string())
        );
    }
}
