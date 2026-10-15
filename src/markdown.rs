use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::ops::Range;

fn body(source: &str) -> (&str, usize) {
    let mut lines = source.split_inclusive('\n');
    if lines.next().is_some_and(|line| line.trim_end() == "---") {
        let mut end = source.find('\n').map_or(source.len(), |n| n + 1);
        for line in lines {
            end += line.len();
            if matches!(line.trim_end(), "---" | "...") {
                return (&source[end..], end);
            }
        }
    }
    (source, 0)
}

pub(crate) fn prose_ranges(source: &str) -> Vec<Range<usize>> {
    let (text, base) = body(source);
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut excluded = 0;
    for (event, range) in Parser::new(text).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::Image { .. } | Tag::HtmlBlock) => excluded += 1,
            Event::End(TagEnd::CodeBlock | TagEnd::Image | TagEnd::HtmlBlock) => excluded -= 1,
            Event::Text(_) | Event::SoftBreak | Event::HardBreak if excluded == 0 => {
                let range = range.start + base..range.end + base;
                if let Some(last) = ranges.last_mut().filter(|last| last.end == range.start) {
                    last.end = range.end;
                } else {
                    ranges.push(range);
                }
            }
            _ => {}
        }
    }
    ranges
}

pub(crate) struct Heading {
    pub title: String,
    pub range: Range<usize>,
    pub content: bool,
    level: u8,
}

pub(crate) fn headings(source: &str) -> Vec<Heading> {
    let (text, base) = body(source);
    let mut headings: Vec<Heading> = Vec::new();
    let mut in_heading = false;
    let mut active: Vec<usize> = Vec::new();
    for (event, range) in Parser::new(text).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let level = level as u8;
                while active.last().is_some_and(|&i| headings[i].level >= level) {
                    active.pop();
                }
                active.push(headings.len());
                in_heading = true;
                headings.push(Heading {
                    title: String::new(),
                    range: range.start + base..range.end + base,
                    content: false,
                    level,
                });
            }
            Event::End(TagEnd::Heading(_)) => in_heading = false,
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = headings.last_mut()
                    && in_heading
                {
                    heading.title.push_str(&text);
                }
                if !in_heading && !text.trim().is_empty() {
                    for &index in &active {
                        headings[index].content = true;
                    }
                }
            }
            _ => {}
        }
    }
    headings
}
