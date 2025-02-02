use failure::{ensure, Error, Fail};
use select::document::Document;
use select::node::Node;
use select::predicate::{Class, Name, Predicate};

use std::collections::HashMap;
use std::f32::consts::E;
use std::str::FromStr;

use crate::model::{
    attribute::{Attribute, Attributes},
    clan::Clan,
    class::{ClassInfo, ClassType, Classes},
    datacenter::Datacenter,
    gender::Gender,
    race::Race,
    server::Server,
    util::load_url,
};

use super::gear::{Gear, GearSet, GearSlot, Slot};
use super::language::Language;

/// Represents ways in which a search over the HTML data might go wrong.
#[derive(Fail, Debug)]
pub enum SearchError {
    /// A search for a node that was required turned up empty.
    #[fail(display = "Node not found: {}", _0)]
    NodeNotFound(String),
    /// A node was found, but the data inside it was malformed.
    #[fail(display = "Invalid data found while parsing '{}'", _0)]
    InvalidData(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct CharInfo {
    race: Race,
    clan: Clan,
    gender: Gender,
}

struct HomeInfo {
    server: Server,
    datacenter: Datacenter,
}

/// Takes a Document and a search expression, and will return
/// a `SearchError` if it is not found. Otherwise it will return
/// the found node.
macro_rules! ensure_node {
    ($doc:ident, $search:expr) => {{
        ensure_node!($doc, $search, 0)
    }};

    ($doc:ident, $search:expr, $nth:expr) => {{
        let node = $doc.find($search).nth($nth);
        ensure!(
            node.is_some(),
            SearchError::NodeNotFound(
                stringify!($search).to_string() + "(" + stringify!($nth) + ")"
            )
        );
        node.unwrap()
    }};
}

/// Holds all the data for a profile retrieved via Lodestone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LightProfile {
    /// The id associated with the profile
    pub user_id: u32,
    /// The character's in-game name.
    pub name: String,
    /// Which server the character is in.
    pub server: Server,
    /// Which datacenter the character is in.
    pub datacenter: Datacenter,
    /// A URL to the character's face portrait.
    pub face_portrait_url: String,
}

impl LightProfile {
    pub fn create_from(node: &Node<'_>) -> Result<Self, Error> {
        let home_info = Self::parse_home(node)?;

        Ok(Self {
            user_id: Self::parse_user_id(node)?,
            name: Self::parse_name(node)?,
            server: home_info.server,
            datacenter: home_info.datacenter,
            face_portrait_url: Self::parse_image_url(node, "entry__chara__face")?,
        })
    }

    fn parse_user_id(node: &Node<'_>) -> Result<u32, Error> {
        let href = ensure_node!(node, Class("entry__link")).attr("href");
        match href {
            Some(href) => {
                let digits = href
                    .chars()
                    .skip_while(|ch| !ch.is_digit(10))
                    .take_while(|ch| ch.is_digit(10))
                    .collect::<String>();
                Ok(digits.parse::<u32>()?)
            }
            None => Err(SearchError::InvalidData("missing user profile href".into()).into()),
        }
    }

    fn parse_home(node: &Node<'_>) -> Result<HomeInfo, Error> {
        let text = ensure_node!(node, Class("entry__world")).text();
        let parts = text.split(" [").collect::<Vec<&str>>();
        ensure!(
            parts.len() == 2,
            SearchError::InvalidData("entry__world".into())
        );
        Ok(HomeInfo {
            server: Server::from_str(parts[0])?,
            datacenter: Datacenter::from_str(parts[1].trim_end_matches(']'))?,
        })
    }

    fn parse_name(node: &Node<'_>) -> Result<String, Error> {
        Ok(ensure_node!(node, Class("entry__name")).text())
    }

    fn parse_image_url(node: &Node<'_>, class: &str) -> Result<String, Error> {
        let img_src = ensure_node!(node, Class(class).descendant(Name("img"))).attr("src");
        match img_src {
            Some(src) => Ok(src.to_string()),
            None => Err(SearchError::InvalidData("missing image source".into()).into()),
        }
    }
}

/// Holds all the data for a profile retrieved via Lodestone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    /// The id associated with the profile
    pub user_id: u32,
    /// The profile's associated Free Company
    pub free_company: Option<String>,
    /// The profile's title
    pub title: Option<String>,
    /// The character's in-game name.
    pub name: String,
    /// The character's nameday
    pub nameday: String,
    /// The character's guardian
    pub guardian: String,
    /// The character's city state
    pub city_state: String,
    /// Which server the character is in.
    pub server: Server,
    /// Which datacenter the character is in.
    pub datacenter: Datacenter,
    /// What race the character is.
    pub race: Race,
    /// One of the two clans associated with their race.
    pub clan: Clan,
    /// Character's gender.
    pub gender: Gender,
    /// Max HP.
    pub hp: u32,
    /// Max MP.
    pub mp: u32,
    /// A list of attributes and their values.
    pub attributes: Attributes,
    /// A map of the item for each gear slot.
    pub gear: GearSet,
    /// A URL to the character's face portrait.
    pub face_portrait_url: String,
    /// A URL to the character's portrait.
    pub portrait_url: String,
    /// A list of classes and their corresponding levels.
    classes: Classes,
}

impl Profile {
    /// Gets a profile for a user given their lodestone user id.
    ///
    /// If you don't have the id, it is possible to use a
    /// `SearchBuilder` in order to find their profile directly.
    pub fn get(user_id: u32) -> Result<Self, Error> {
        let main_doc = load_url(user_id, None)?;
        let classes_doc = load_url(user_id, Some("class_job"))?;

        //  Holds the string for Race, Clan, and Gender in that order
        let char_info = Self::parse_char_info(&main_doc)?;

        //  Holds the string for Server, Datacenter in that order
        let home_info = Self::parse_home_info(&main_doc)?;

        let (hp, mp) = Self::parse_char_param(&main_doc)?;

        Ok(Self {
            user_id,
            free_company: Self::parse_free_company(&main_doc),
            title: Self::parse_title(&main_doc),
            name: Self::parse_name(&main_doc)?,
            nameday: Self::parse_nameday(&main_doc)?,
            guardian: Self::parse_guardian(&main_doc)?,
            city_state: Self::parse_city_state(&main_doc)?,
            server: home_info.server,
            datacenter: home_info.datacenter,
            race: char_info.race,
            clan: char_info.clan,
            gender: char_info.gender,
            hp,
            mp,
            attributes: Self::parse_attributes(&main_doc)?,
            gear: Self::parse_gear(&main_doc)?,
            face_portrait_url: Self::parse_image_url(&main_doc, "frame__chara__face")?,
            portrait_url: Self::parse_image_url(&main_doc, "character__detail__image")?,
            classes: Self::parse_classes(&classes_doc)?,
        })
    }

    /// Get the level of a specific class for this profile.
    ///
    /// This can be used to query whether or not a job is unlocked.
    /// For instance if Gladiator is below 30, then Paladin will
    /// return None. If Paladin is unlocked, both Gladiator and
    /// Paladin will return the same level.
    pub fn level(&self, class: ClassType) -> Option<u32> {
        match self.class_info(class) {
            Some(v) => Some(v.level),
            None => None,
        }
    }

    /// Gets this profile's data for a given class
    pub fn class_info(&self, class: ClassType) -> Option<ClassInfo> {
        self.classes.get(class)
    }

    /// Borrows the full map of classes, e.g. for iteration in calling code
    pub fn all_class_info(&self) -> &Classes {
        &self.classes
    }

    fn parse_free_company(doc: &Document) -> Option<String> {
        match doc.find(Class("character__freecompany__name")).next() {
            Some(node) => Some(
                node.text()
                    .strip_prefix("Free Company")
                    .unwrap_or(&node.text())
                    .to_string(),
            ),
            None => None,
        }
    }

    fn parse_title(doc: &Document) -> Option<String> {
        match doc.find(Class("frame__chara__title")).next() {
            Some(node) => Some(node.text()),
            None => None,
        }
    }

    fn parse_name(doc: &Document) -> Result<String, Error> {
        Ok(ensure_node!(doc, Class("frame__chara__name")).text())
    }

    fn parse_nameday(doc: &Document) -> Result<String, Error> {
        Ok(ensure_node!(doc, Class("character-block__birth")).text())
    }

    fn parse_guardian(doc: &Document) -> Result<String, Error> {
        Ok(ensure_node!(doc, Class("character-block__name"), 1).text())
    }

    fn parse_city_state(doc: &Document) -> Result<String, Error> {
        Ok(ensure_node!(doc, Class("character-block__name"), 2).text())
    }

    fn parse_home_info(doc: &Document) -> Result<HomeInfo, Error> {
        let text = ensure_node!(doc, Class("frame__chara__world")).text();
        let mut server = text.split("\u{A0}").next();

        ensure!(
            server.is_some(),
            SearchError::InvalidData("Could not find server/datacenter string.".into())
        );

        // String comes in format Server [Datacenter]
        let home_info = server
            .unwrap()
            .split_whitespace()
            .map(|e| e.replace(&['[', ']'], ""))
            .collect::<Vec<String>>();

        Ok(HomeInfo {
            server: Server::from_str(&home_info[0])?,
            datacenter: Datacenter::from_str(&home_info[1])?,
        })
    }

    fn parse_char_info(doc: &Document) -> Result<CharInfo, Error> {
        let char_block = {
            let mut block = ensure_node!(doc, Class("character-block__name")).inner_html();
            block = block.replace(" ", "_");
            block = block.replace("<br>", " ");
            block.replace("_/_", " ")
        };

        let char_info = char_block
            .split_whitespace()
            .map(|e| e.replace("_", " "))
            .map(|e| e.into())
            .collect::<Vec<String>>();

        ensure!(
            char_info.len() == 3 || char_info.len() == 4,
            SearchError::InvalidData("character block name".into())
        );

        //  If the length is 4, then the race is "Au Ra"
        if char_info.len() == 4 {
            Ok(CharInfo {
                race: Race::Aura,
                clan: Clan::from_str(&char_info[2])?,
                gender: Gender::from_str(&char_info[3])?,
            })
        } else {
            Ok(CharInfo {
                race: Race::from_str(&char_info[0])?,
                clan: Clan::from_str(&char_info[1])?,
                gender: Gender::from_str(&char_info[2])?,
            })
        }
    }

    fn parse_char_param(doc: &Document) -> Result<(u32, u32), Error> {
        let attr_block = ensure_node!(doc, Class("character__param"));
        let mut hp = None;
        let mut mp = None;
        for item in attr_block.find(Name("li")) {
            if item
                .find(Class("character__param__text__hp--en-us"))
                .count()
                == 1
            {
                hp = Some(ensure_node!(item, Name("span")).text().parse::<u32>()?);
            } else if item
                .find(Class("character__param__text__mp--en-us"))
                .count()
                == 1
                || item
                    .find(Class("character__param__text__gp--en-us"))
                    .count()
                    == 1
                || item
                    .find(Class("character__param__text__cp--en-us"))
                    .count()
                    == 1
            {
                // doh/dol jobs change the css now to show GP/CP. if any is present, store as mp
                mp = Some(ensure_node!(item, Name("span")).text().parse::<u32>()?);
            } else {
                continue;
            }
        }
        ensure!(
            hp.is_some() && mp.is_some(),
            SearchError::InvalidData("character__param".into())
        );

        Ok((hp.unwrap(), mp.unwrap()))
    }

    fn parse_attributes(doc: &Document) -> Result<Attributes, Error> {
        let block = ensure_node!(doc, Class("character__profile__data"));
        let mut attributes = Attributes::new();
        for item in block.find(Name("tr")) {
            let name = ensure_node!(item, Name("span")).text();
            let value = Attribute {
                level: ensure_node!(item, Name("td")).text().parse::<u16>()?,
            };
            attributes.insert(name, value);
        }
        Ok(attributes)
    }

    fn parse_gear(doc: &Document) -> Result<GearSet, Error> {
        let mut gear = GearSet::new();
        let class_to_slot = HashMap::from([
            ("icon-c--0", Slot::PrimaryWeapon),
            ("icon-c--1", Slot::SecondaryWeapon),
            ("icon-c--2", Slot::Head),
            ("icon-c--3", Slot::Body),
            ("icon-c--4", Slot::Hands),
            ("icon-c--6", Slot::Legs),
            ("icon-c--7", Slot::Feet),
            ("icon-c--8", Slot::Earrings),
            ("icon-c--9", Slot::Necklace),
            ("icon-c--10", Slot::Bracelets),
            ("icon-c--11", Slot::Ring1),
            ("icon-c--12", Slot::Ring2),
            ("icon-c--13", Slot::Soul),
            ("icon-c--13", Slot::Soul),
            ("icon-c--glasses", Slot::Glasses),
        ]);
        for (class, slot) in class_to_slot.iter() {
            if let Some(node) = doc.find(Class(*class)).next() {
                if node.text() == "" {
                    continue;
                }

                let gear_link =
                    ensure_node!(node, Class("db-tooltip__bt_item_detail").child(Name("a")));
                let node = ensure_node!(node, Class("db-tooltip__item__txt"));
                let gear_slot = GearSlot {
                    gear: Gear {
                        lodestone_id: Self::parse_gear_link(gear_link.attr("href"))?,
                        name: ensure_node!(node, Class("db-tooltip__item__name")).text(),
                    },
                    glamour: match node.find(Class("db-tooltip__item__mirage")).next() {
                        Some(glamour_data) => {
                            let glamour_link =
                                ensure_node!(glamour_data, Class("db-tooltip__item__mirage__btn"));
                            Some(Gear {
                                lodestone_id: Self::parse_gear_link(glamour_link.attr("href"))?,
                                name: glamour_data.text(),
                            })
                        }
                        None => None,
                    },
                };
                gear.insert(*slot, gear_slot);
            }
        }
        Ok(gear)
    }

    fn parse_gear_link(href: Option<&str>) -> Result<String, Error> {
        match href {
            Some(href) => {
                // expecting something like href="/lodestone/playguide/db/item/23c482f7f46/"
                let parts = href.split('/').collect::<Vec<&str>>();
                if parts.len() != 7 {
                    return Err(SearchError::InvalidData("invalid gear link".into()).into());
                }
                let id = parts[5];
                Ok(id.to_string())
            }
            None => Err(SearchError::InvalidData("missing gear link".into()).into()),
        }
    }

    fn parse_image_url(doc: &Document, class: &str) -> Result<String, Error> {
        let img_src = ensure_node!(doc, Class(class).descendant(Name("img"))).attr("src");
        match img_src {
            Some(src) => Ok(src.to_string()),
            None => Err(SearchError::InvalidData("missing image source".into()).into()),
        }
    }

    fn parse_classes(doc: &Document) -> Result<Classes, Error> {
        let mut classes = Classes::new();

        for list in doc.find(Class("character__content")).take(4) {
            for item in list.find(Name("li")) {
                let name = ensure_node!(item, Class("character__job__name")).text();
                let classinfo = match ensure_node!(item, Class("character__job__level"))
                    .text()
                    .as_str()
                {
                    "-" => None,
                    level => {
                        let text = ensure_node!(item, Class("character__job__exp")).text();
                        let mut parts = text.split(" / ");
                        let current_xp = parts.next();
                        ensure!(
                            current_xp.is_some(),
                            SearchError::InvalidData("character__job__exp".into())
                        );
                        let max_xp = parts.next();
                        ensure!(
                            max_xp.is_some(),
                            SearchError::InvalidData("character__job__exp".into())
                        );
                        Some(ClassInfo {
                            level: level.parse()?,
                            current_xp: match current_xp.unwrap() {
                                "--" => None,
                                value => Some(value.replace(",", "").parse()?),
                            },
                            max_xp: match max_xp.unwrap() {
                                "--" => None,
                                value => Some(value.replace(",", "").parse()?),
                            },
                        })
                    }
                };

                //  For classes that have multiple titles (e.g., Paladin / Gladiator), grab the first one.
                let name = name.split(" / ").next();
                ensure!(
                    name.is_some(),
                    SearchError::InvalidData("character__job__name".into())
                );
                let class = ClassType::from_str(&name.unwrap())?;

                //  If the class added was a secondary job, then associated that level
                //  with its lower level counterpart as well. This makes returning the
                //  level for a particular grouping easier at the cost of memory.
                match class {
                    ClassType::Paladin => classes.insert(ClassType::Gladiator, classinfo),
                    ClassType::Warrior => classes.insert(ClassType::Marauder, classinfo),
                    ClassType::WhiteMage => classes.insert(ClassType::Conjurer, classinfo),
                    ClassType::Monk => classes.insert(ClassType::Pugilist, classinfo),
                    ClassType::Dragoon => classes.insert(ClassType::Lancer, classinfo),
                    ClassType::Ninja => classes.insert(ClassType::Rogue, classinfo),
                    ClassType::Bard => classes.insert(ClassType::Archer, classinfo),
                    ClassType::BlackMage => classes.insert(ClassType::Thaumaturge, classinfo),
                    ClassType::Summoner => classes.insert(ClassType::Arcanist, classinfo),
                    _ => (),
                }

                classes.insert(class, classinfo);
            }
        }

        Ok(classes)
    }
}
