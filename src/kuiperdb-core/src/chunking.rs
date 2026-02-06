use anyhow::Result;
use tiktoken_rs::cl100k_base;

/// Trait for different chunking strategies
pub trait Chunker: Send + Sync {
    fn chunk(&self, text: &str, chunk_size: usize, overlap: usize) -> Result<Vec<String>>;
    fn count_tokens(&self, text: &str) -> Result<usize>;
}

/// Fixed token-based chunker using tiktoken
pub struct FixedTokenChunker {
    bpe: tiktoken_rs::CoreBPE,
}

impl FixedTokenChunker {
    pub fn new() -> Result<Self> {
        let bpe = cl100k_base()?;
        Ok(Self { bpe })
    }
}

impl Chunker for FixedTokenChunker {
    fn chunk(&self, text: &str, chunk_size: usize, overlap: usize) -> Result<Vec<String>> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        // Tokenize the entire text
        let tokens = self.bpe.encode_with_special_tokens(text);

        // If text is smaller than chunk_size, return as single chunk
        if tokens.len() <= chunk_size {
            return Ok(vec![text.to_string()]);
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        // Ensure overlap is smaller than chunk_size
        let effective_overlap = overlap.min(chunk_size.saturating_sub(1));

        while start < tokens.len() {
            let end = std::cmp::min(start + chunk_size, tokens.len());
            let chunk_tokens = &tokens[start..end];

            // Decode tokens back to text
            let chunk_text = self.bpe.decode(chunk_tokens.to_vec())?;
            chunks.push(chunk_text);

            // Move to next chunk with overlap
            if end >= tokens.len() {
                break;
            }

            // Calculate next start position
            start = if effective_overlap > 0 {
                end.saturating_sub(effective_overlap)
            } else {
                end
            };

            // Safety check to prevent infinite loop
            if start >= end {
                break;
            }
        }

        Ok(chunks)
    }

    fn count_tokens(&self, text: &str) -> Result<usize> {
        let tokens = self.bpe.encode_with_special_tokens(text);
        Ok(tokens.len())
    }
}

/// Custom chunker stub - allows user to implement their own logic
pub struct CustomChunker;

impl Chunker for CustomChunker {
    fn chunk(&self, text: &str, chunk_size: usize, _overlap: usize) -> Result<Vec<String>> {
        // Stub: Simple character-based chunking for now
        // Users can replace this with semantic/paragraph-aware chunking
        if text.is_empty() {
            return Ok(vec![]);
        }

        let chars_per_chunk = chunk_size * 4; // Rough approximation: 4 chars per token
        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = std::cmp::min(start + chars_per_chunk, text.len());
            chunks.push(text[start..end].to_string());
            start = end;
        }

        Ok(chunks)
    }

    fn count_tokens(&self, text: &str) -> Result<usize> {
        // Rough approximation: 4 characters per token
        Ok(text.len() / 4)
    }
}

/// Markdown-aware chunker that splits by horizontal rules (---, ___, ***)
/// Falls back to paragraph chunking if sections are too large
pub struct MarkdownChunker {
    bpe: tiktoken_rs::CoreBPE,
}

impl MarkdownChunker {
    pub fn new() -> Result<Self> {
        let bpe = cl100k_base()?;
        Ok(Self { bpe })
    }

    /// Split text into markdown sections using horizontal rules (---, ___, ***) as delimiters
    fn split_by_sections(&self, text: &str) -> Vec<(usize, String)> {
        let mut sections = Vec::new();
        let mut current_section = String::new();

        for line in text.lines() {
            // Check if line is a horizontal rule (---, ___, ***)
            let trimmed_line = line.trim();
            let is_hr = if trimmed_line.len() >= 3 {
                if let Some(c) = trimmed_line.chars().next() {
                    (c == '-' || c == '_' || c == '*') && trimmed_line.chars().all(|ch| ch == c)
                } else {
                    false
                }
            } else {
                false
            };
            
            if is_hr {
                // Horizontal rule marks end of section - save current section
                if !current_section.is_empty() {
                    sections.push((0, Self::clean_section(&current_section)));
                    current_section.clear();
                }
            } else {
                // Add line to current section
                current_section.push_str(line);
                current_section.push('\n');
            }
        }

        // Add final section
        if !current_section.is_empty() {
            sections.push((0, Self::clean_section(&current_section)));
        }

        sections
    }
    
    /// Clean section by trimming whitespace and removing empty lines from start/end
    fn clean_section(text: &str) -> String {
        text.trim().to_string()
    }

    /// Split large section by paragraphs
    fn split_by_paragraphs(&self, text: &str, max_tokens: usize) -> Result<Vec<String>> {
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for para in paragraphs {
            let para_tokens = self.count_tokens(para)?;
            let current_tokens = self.count_tokens(&current_chunk)?;

            // If single paragraph is too large, split it with fixed token chunker
            if para_tokens > max_tokens {
                // Save current chunk if not empty
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk.clear();
                }
                
                // Split large paragraph with fixed token chunker
                let para_chunks = FixedTokenChunker::new()?.chunk(para, max_tokens, 50)?;
                chunks.extend(para_chunks);
                continue;
            }

            // Check if adding this paragraph would exceed limit
            if current_tokens + para_tokens > max_tokens && !current_chunk.is_empty() {
                // Save current chunk and start new one
                chunks.push(current_chunk.trim().to_string());
                current_chunk = format!("{}\n\n", para);
            } else {
                // Add paragraph to current chunk
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(para);
            }
        }

        // Add final chunk
        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        Ok(chunks)
    }
}

impl Chunker for MarkdownChunker {
    fn chunk(&self, text: &str, chunk_size: usize, _overlap: usize) -> Result<Vec<String>> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        // Split by markdown sections first
        let sections = self.split_by_sections(text);
        
        let mut chunks = Vec::new();

        for (_level, section) in sections {
            let section_tokens = self.count_tokens(&section)?;

            if section_tokens <= chunk_size {
                // Section fits in one chunk
                chunks.push(section);
            } else {
                // Section is too large, split by paragraphs
                let para_chunks = self.split_by_paragraphs(&section, chunk_size)?;
                chunks.extend(para_chunks);
            }
        }

        Ok(chunks)
    }

    fn count_tokens(&self, text: &str) -> Result<usize> {
        let tokens = self.bpe.encode_with_special_tokens(text);
        Ok(tokens.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_token_chunker_empty() {
        let chunker = FixedTokenChunker::new().unwrap();
        let chunks = chunker.chunk("", 512, 50).unwrap();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_fixed_token_chunker_small_text() {
        let chunker = FixedTokenChunker::new().unwrap();
        let text = "This is a small text.";
        let chunks = chunker.chunk(text, 512, 50).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }
    
    #[test]
    fn test_markdown_chunker_splits_by_horizontal_rules() {
        let chunker = MarkdownChunker::new().unwrap();
        let text = "# Title\nSome content\n\n---\n\n## Section\nMore content";
        let chunks = chunker.chunk(text, 512, 0).unwrap();
        assert_eq!(chunks.len(), 2, "Should split into 2 chunks at ---");
        assert!(chunks[0].contains("Title"), "First chunk should contain title");
        assert!(chunks[1].contains("Section"), "Second chunk should contain section");
        assert!(!chunks[0].contains("---"), "Chunks should not contain the delimiter");
        assert!(!chunks[1].contains("---"), "Chunks should not contain the delimiter");
    }

    #[test]
    fn test_fixed_token_chunker_count_tokens() {
        let chunker = FixedTokenChunker::new().unwrap();
        let text = "This is a test.";
        let count = chunker.count_tokens(text).unwrap();
        assert!(count > 0);
        assert!(count < 20); // Should be ~4-5 tokens
    }

    #[test]
    fn test_fixed_token_chunker_large_text() {
        let chunker = FixedTokenChunker::new().unwrap();

        // Create text larger than chunk size
        let text = "word ".repeat(1000); // ~1000 tokens
        let chunks = chunker.chunk(&text, 512, 50).unwrap();

        // Should create multiple chunks
        assert!(chunks.len() > 1);

        // Each chunk should be non-empty
        for chunk in &chunks {
            assert!(!chunk.is_empty());
        }
    }

    #[test]
    fn test_fixed_token_chunker_overlap() {
        let chunker = FixedTokenChunker::new().unwrap();

        // Create text that will be split
        let text = "word ".repeat(600); // ~600 tokens
        let chunks = chunker.chunk(&text, 512, 50).unwrap();

        // Should have overlap between chunks
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_custom_chunker_stub() {
        let chunker = CustomChunker;
        let text = "a".repeat(10000);
        let chunks = chunker.chunk(&text, 512, 0).unwrap();

        assert!(chunks.len() > 1);
        assert!(!chunks[0].is_empty());
    }

    #[test]
    fn test_custom_chunker_token_count() {
        let chunker = CustomChunker;
        let text = "This is a test with approximately 40 characters";
        let count = chunker.count_tokens(text).unwrap();

        // Should be ~11-12 tokens (48 chars / 4)
        assert!(
            (11..=12).contains(&count),
            "Expected 11-12 tokens, got {}",
            count
        );
    }
}
