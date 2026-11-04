//! Resume content (ported from the standalone resume app) and its page.
//!
//! Public version: location is the country only, no street address, phone
//! number or email.

use maud::{html, Markup};

use crate::{
    site::Config,
    templates::{self, Page},
};

pub(crate) struct Job {
    pub(crate) place: &'static str,
    pub(crate) start: u16,
    /// `None` while ongoing.
    pub(crate) end: Option<u16>,
    pub(crate) title: &'static str,
    pub(crate) tasks: &'static [&'static str],
    pub(crate) stack: &'static [&'static str],
}

pub(crate) struct Education {
    pub(crate) place: &'static str,
    pub(crate) start: u16,
    pub(crate) end: u16,
    pub(crate) degree: &'static str,
}

/// Headline and roles at the top of the resume.
pub(crate) const HEADLINE: &str = "Software Engineer";
pub(crate) const ROLES: &[&str] = &["Developer", "Entrepreneur", "Writer"];

pub(crate) const LOCATION: &str = "Finland";

/// Spoken languages as shown, with their BCP 47 codes for structured data.
pub(crate) const LANGUAGES: &[(&str, &str)] =
    &[("Native Finnish", "fi"), ("Proficient English", "en")];

pub(crate) const SKILLS: &[&str] = &["DevOps", "Functional programming", "Infographics", "Web"];

pub(crate) const INTERESTS: &[&str] = &[
    "Cognitive sciences",
    "Cybersecurity",
    "Mindfulness",
    "Reading",
    "Writing",
];

pub(crate) const RESPONSIBILITIES: &[&str] = &["Member Secretary of Tai Chi Chuan ry"];

pub(crate) const OVERVIEW: &[&str] = &[
    "Open source and hacker culture inspired me to study computer science. I am self-employed \
     software engineer, and I've worked in multicultural teams in Finnish and English.",
    "I am fascinated by artificial intelligence and especially natural language processing that \
     allows human-like interfacing to modern computation. I am equally comfortable working at the \
     command line and developing visualization tools with interactive analysis and reporting.",
    "My strengths are open approach to challenges, observational experimentation and imaginative \
     adaptability. By nature I am curious, diligent, conscientious and thoughtful.",
    "I enjoy exploring technical details, discovering scientific breakthroughs, understanding \
     human psychology, improving my creative writing, learning languages and simply enjoying long \
     walks in nature.",
    "I also appreciate the option to work remotely on my own schedule while following my other \
     passion: Tai Chi.",
];

pub(crate) const CAREER_SUMMARY: &str = "I've worked on bioinformatics, healthcare, laboratory \
     testing, scientific research, telecommunications and welfare, in domains such as analysis, \
     configuration, reporting, time-tracking, translation, usability, validation, verification \
     and visualization.";

pub(crate) const JOBS: &[Job] = &[
    Job {
        place: "Self-Employed",
        start: 2020,
        end: None,
        title: "Software Developer & Tai Chi Teacher",
        tasks: &[],
        stack: &["Nix", "Rust", "SQLite"],
    },
    Job {
        place: "Finbiosoft",
        start: 2015,
        end: Some(2020),
        title: "Lead Frontend Developer",
        tasks: &["Documentation", "Reports", "Usability"],
        stack: &["Docker", "F#", "TypeScript"],
    },
    Job {
        place: "MediSapiens",
        start: 2010,
        end: Some(2015),
        title: "Software Developer ≫ Senior Software Architect",
        tasks: &[],
        stack: &["JavaScript", "MongoDB", "Redis"],
    },
    Job {
        place: "Nice Business Solutions",
        start: 2009,
        end: Some(2010),
        title: "System Administration & Software Support",
        tasks: &[],
        stack: &["Django", "PostgreSQL"],
    },
    Job {
        place: "Rossum",
        start: 2008,
        end: Some(2009),
        title: "Software Developer",
        tasks: &[],
        stack: &["Java", "jQuery"],
    },
    Job {
        place: "Tieto",
        start: 2006,
        end: Some(2008),
        title: "Software Designer",
        tasks: &[],
        stack: &["C#", ".Net", "T-SQL"],
    },
    Job {
        place: "QuickClic Finland",
        start: 2005,
        end: Some(2006),
        title: "Software Designer",
        tasks: &[],
        stack: &["Linux", "MySQL", "Python"],
    },
    Job {
        place: "Lahti Region Educational Consortium",
        start: 2004,
        end: Some(2005),
        title: "Thesis Intern",
        tasks: &[],
        stack: &["AD", "Windows"],
    },
];

pub(crate) const EDUCATION: &[Education] = &[Education {
    place: "Lahti University of Applied Sciences",
    start: 2001,
    end: 2007,
    degree: "Bachelor of Software Engineering",
}];

pub(crate) const PAGE: Page = Page {
    path: "/resume",
    title: "Resume — Timo Mäkelä",
    description: "Resume of Timo Mäkelä, self-employed software engineer in Finland: two decades \
         of software for healthcare, bioinformatics and telecoms, in Rust, Nix and F#.",
};

fn years(start: u16, end: Option<u16>) -> String {
    match end {
        Some(end) => format!("{start} – {end}"),
        None => format!("{start} – present"),
    }
}

fn list(items: &[&str]) -> Markup {
    html! { ul { @for item in items { li { (item) } } } }
}

pub(crate) fn page(config: &Config) -> Markup {
    templates::layout(
        config,
        &PAGE,
        &html! {
            div.resume {
                aside {
                    p.location { (LOCATION) }
                    h2 { "Languages" }
                    ul { @for (language, _) in LANGUAGES { li { (language) } } }
                    h2 { "Skills" }
                    (list(SKILLS))
                    h2 { "Interests" }
                    (list(INTERESTS))
                    h2 { "Responsibilities" }
                    (list(RESPONSIBILITIES))
                }
                div.resume-main {
                    header {
                        h1 { (crate::site::NAME) }
                        p.headline { (HEADLINE) }
                        ul.roles { @for role in ROLES { li { (role) } } }
                    }
                    section {
                        h2 { "Overview" }
                        @for paragraph in OVERVIEW { p { (paragraph) } }
                    }
                    section {
                        h2 { "Career" }
                        p { (CAREER_SUMMARY) }
                        ol.timeline {
                            @for job in JOBS {
                                li {
                                    span.years { (years(job.start, job.end)) }
                                    div {
                                        h3 { (job.place) }
                                        p.title { (job.title) }
                                        @if !job.tasks.is_empty() {
                                            p.tasks { (job.tasks.join(" · ")) }
                                        }
                                        ul.tags { @for tech in job.stack { li { (tech) } } }
                                    }
                                }
                            }
                        }
                    }
                    section {
                        h2 { "Education" }
                        ol.timeline {
                            @for school in EDUCATION {
                                li {
                                    span.years { (years(school.start, Some(school.end))) }
                                    div {
                                        h3 { (school.place) }
                                        p.title { (school.degree) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jobs_are_newest_first_and_consistent() {
        for pair in JOBS.windows(2) {
            if let [newer, older] = pair {
                assert!(
                    newer.start >= older.start,
                    "{} before {}",
                    newer.place,
                    older.place
                );
            }
        }
        for job in JOBS {
            assert!(job.end.is_none_or(|end| end >= job.start), "{}", job.place);
        }
    }

    #[test]
    fn only_the_current_job_is_ongoing() {
        assert_eq!(JOBS.iter().filter(|job| job.end.is_none()).count(), 1);
    }

    #[test]
    fn page_has_no_private_contact_details() {
        let html = page(&Config {
            site_url: "https://example.com".to_string(),
        })
        .into_string();
        // Generic patterns only: the actual address must never be in the repo.
        for private in ["mailto:", "tel:", "+358"] {
            assert!(!html.contains(private), "resume page contains {private}");
        }
    }

    #[test]
    fn open_ended_years_read_present() {
        assert_eq!(years(2020, None), "2020 – present");
        assert_eq!(years(2015, Some(2020)), "2015 – 2020");
    }
}
