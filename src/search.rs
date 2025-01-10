use failure::Error;
use select::document::Document;
use select::predicate::{Class, Predicate};

use crate::model::datacenter::Datacenter;
use crate::model::domain::Domain;
use crate::model::gc::GrandCompany;
use crate::model::language::Language;
use crate::model::profile::{LightProfile, Profile};
use crate::model::server::Server;
use crate::CLIENT;

use std::collections::HashSet;
use std::fmt::Write;

#[derive(Clone, Debug, Default)]
pub struct SearchBuilder {
    domain: Option<Domain>,
    server: Option<Server>,
    datacenter: Option<Datacenter>,
    character: Option<String>,
    lang: HashSet<Language>,
    gc: HashSet<GrandCompany>,
}

impl SearchBuilder {
    pub fn new() -> Self {
        SearchBuilder {
            ..Default::default()
        }
    }

    /// Builds the search and executes it, returning a list of profiles
    /// that match the given criteria.
    fn send_common(self) -> Result<Document, Error> {
        let mut url = format!(
            "https://{}.finalfantasyxiv.com/lodestone/character/?",
            self.domain.unwrap_or(Domain::NorthAmerica).to_string()
        );

        if let Some(name) = self.character {
            let _ = write!(url, "q={}&", name);
        }

        if let Some(dc) = self.datacenter {
            let _ = write!(url, "worldname=_dc_{}&", dc);
        }

        if let Some(s) = self.server {
            let _ = write!(url, "worldname={}&", s);
        }

        self.lang.iter().for_each(|lang| {
            let _ = match lang {
                Language::Japanese => write!(url, "blog_lang=ja&"),
                Language::English => write!(url, "blog_lang=en&"),
                Language::German => write!(url, "blog_lang=de&"),
                Language::French => write!(url, "blog_lang=fr&"),
            };
        });

        self.gc.iter().for_each(|gc| {
            let _ = match gc {
                GrandCompany::Unaffiliated => write!(url, "gcid=0&"),
                GrandCompany::Maelstrom => write!(url, "gcid=1&"),
                GrandCompany::TwinAdder => write!(url, "gcid=2&"),
                GrandCompany::ImmortalFlames => write!(url, "gcid=3&"),
            };
        });

        let url = url.trim_end_matches('&');

        let response = CLIENT.get(url).send()?;
        let text = response.text()?;
        let doc = Document::from(text.as_str());

        Ok(doc)
    }

    /// Builds the search and executes it, returning a list of profiles
    /// that match the given criteria.
    pub fn send(self) -> Result<Vec<Profile>, Error> {
        Ok(self
            .send_common()?
            .find(Class("entry__link"))
            .filter_map(|node| {
                node.attr("href")
                    .and_then(|text| {
                        let digits = text
                            .chars()
                            .skip_while(|ch| !ch.is_digit(10))
                            .take_while(|ch| ch.is_digit(10))
                            .collect::<String>();

                        digits.parse::<u32>().ok()
                    })
                    .and_then(|id| {
                        let profile = Profile::get(id);

                        profile.ok()
                    })
            })
            .collect())
    }

    /// Builds the search and executes it, returning a list of profiles
    /// that match the given criteria.
    pub fn send_light(self) -> Result<Vec<LightProfile>, Error> {
        Ok(self
            .send_common()?
            .find(Class("ldst__main").descendant(Class("entry")))
            .filter_map(|node| match LightProfile::create_from(&node) {
                Ok(profile) => Some(profile),
                Err(e) => {
                    println!("{:?}", node);
                    println!("{:?}", e);
                    None
                }
            })
            .collect())
    }

    /// Sets the domain to search in. This can only be called once,
    /// and any further calls will simply overwrite the previous domain.
    pub fn domain(mut self, domain: Domain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// A character name to search for. This can only be called once,
    /// and any further calls will simply overwrite the previous name.
    pub fn character(mut self, name: &str) -> Self {
        self.character = Some(name.into());
        self
    }

    /// A datacenter to search in. Mutually exclusive to server.
    /// If a server was specified before calling this method,
    /// it will be replaced by the newer datacenter.
    pub fn datacenter<D: Into<Datacenter>>(mut self, datacenter: D) -> Self {
        self.datacenter = Some(datacenter.into());
        self.server = None;
        self
    }

    /// A server to search in. Mutually exclusive to datacenter.
    /// If a datacenter was specified before calling this method,
    /// it will be replaced by the newer server.
    pub fn server<S: Into<Server>>(mut self, server: S) -> Self {
        self.server = Some(server.into());
        self.datacenter = None;
        self
    }

    /// Which language to filter by.
    /// You can add multiple languages by calling this multiple times.
    pub fn lang<L: Into<Language>>(mut self, lang: L) -> Self {
        self.lang.insert(lang.into());
        self
    }

    /// Which grand company to filter by.
    /// You can add multiple grand company filters by calling this multiple times.
    pub fn grand_company<G: Into<GrandCompany>>(mut self, gc: G) -> Self {
        self.gc.insert(gc.into());
        self
    }
}
