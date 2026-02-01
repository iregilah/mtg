use crate::app::card_attribute::CardAttribute;
use crate::app::card_attribute::CreatureType::Detective;
use crate::app::card_attribute::*;
use crate::app::game_state::GamePhase;
use crate::app::gre::Gre;
use crate::app::gre::gre_structs::ActivatedAbility;
use bitflags::bitflags;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tracing::{debug, info};

const CACOPHONY_SCAMP: &str = "Cacophony Scamp";
const MONASTERY_SWIFTSPEAR: &str = "Monastery Swiftspear";
const ELECTROSTATIC_INFANTRY: &str = "Electrostatic Infantry";
const HEARTFIRE_HERO: &str = "Heartfire Hero";
const FELONIOUS_RAGE: &str = "Felonious Rage";
const MONSTROUS_RAGE: &str = "Monstrous Rage";
const MONSTER_ROLE: &str = "Monster Role";
const BLAZING_CRESCENDO: &str = "Blazing Crescendo";
const DEMONIC_RUCKUS: &str = "Demonic Ruckus";
const BURST_LIGHTNING: &str = "Burst Lightning";
const LIGHTNING_STRIKE: &str = "Lightning Strike";
const MOUNTAIN: &str = "Mountain";
const ROCKFACE_VILLAGE: &str = "Rockface Village";
const GENE_POLLINATOR: &str = "Gene Pollinator";
const LLANOWAR_ELVES: &str = "Llanowar Elves";
const PAWPATCH_RECRUIT: &str = "Pawpatch Recruit";
const BADGERMOLE_CUB: &str = "Badgermole Cub";
const FRENZIED_BALOTH: &str = "Frenzied Baloth";
const HUNTERS_TALENT: &str = "Hunter's Talent";
const OVERPROTECT: &str = "Overprotect";
const SCAVENGING_OOZE: &str = "Scavenging Ooze";
const ARCHDRUIDS_CHARM: &str = "Archdruid's Charm";
const SURRAK_ELUSIVE_HUNTER: &str = "Surrak, Elusive Hunter";
const TERRASYMBIOSIS: &str = "Terrasymbiosis";
const TYVAR_THE_PUMMELER: &str = "Tyvar, the Pummeler";
const OUROBOROID: &str = "Ouroboroid";
const AGONASAUR_REX: &str = "Agonasaur Rex";
const VIVIEN_REID: &str = "Vivien Reid";
const UGIN_EYE_OF_THE_STORMS: &str = "Ugin, Eye of the Storms";
const BA_SING_SE: &str = "Ba Sing Se";
const FOREST: &str = "Forest";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Creature {
    pub power: i32,
    pub toughness: i32,
    pub summoning_sickness: bool,
    pub abilities: Vec<KeywordAbility>,
    pub types: Vec<CreatureType>,

    pub ephemeral_power: i32,
    pub ephemeral_toughness: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardType {
    Creature(Creature),
    Instant,
    Land,
    Enchantment,
    Token,
    Planeswalker,
}

/// ManaCost
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManaCost {
    pub colorless: u32,
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub white: u32,
    pub black: u32,
}

impl ManaCost {
    pub fn new(colorless: u32, red: u32, green: u32, blue: u32, white: u32, black: u32) -> Self {
        ManaCost {
            colorless,
            red,
            green,
            blue,
            white,
            black,
        }
    }
    pub fn free() -> Self {
        ManaCost::new(0, 0, 0, 0, 0, 0)
    }
    pub fn total(&self) -> u32 {
        self.colorless + self.red + self.green + self.blue + self.white + self.black
    }
    pub fn colored(&self) -> u32 {
        self.red + self.green + self.blue + self.white + self.black
    }
}
bitflags! {
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct CardTypeFlags: u32 {
        const NONE         = 0;
        const LAND         = 1 << 0;
        const CREATURE     = 1 << 1;
        const INSTANT      = 1 << 2;
        const SORCERY      = 1 << 3;
        const ENCHANTMENT  = 1 << 4;
        const ARTIFACT     = 1 << 5;
        const PLANESWALKER = 1 << 6;
        const BATTLE       = 1 << 7;
        const TOKEN        = 1 << 8;
    }
}
/// A kártya fő struktúrája.
/// + `type_flags` mező is, bitflags-alapú
#[derive(Debug, Clone)]
pub struct Card {
    pub card_id: u64,
    pub name: String,
    pub card_type: CardType,
    pub type_flags: CardTypeFlags,
    pub mana_cost: ManaCost,
    pub attributes: Vec<Box<dyn CardAttribute>>,
    pub triggers: Vec<Trigger>,
    pub activated_abilities: Vec<ActivatedAbility>,
    pub attached_to: Option<u64>,
}
impl Hash for Card {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // például:
        self.card_id.hash(state);
        self.name.hash(state);

        // card_type -> Hash
        //   Ehhez lentebb "CardType" is implementálja a Hash-t
        self.card_type.hash(state);

        // type_flags bitjei
        self.type_flags.bits().hash(state);

        // mana_cost -> #derive(Hash)
        self.mana_cost.hash(state);

        // DÖNTÉS: triggers & attributes is hashelődjön?
        // Ehhez a Trigger, Condition, SpellFilter, PlayerSelector, stb. mind kell #derive(Hash).
        // Viszont a `Box<dyn CardAttribute>`
        //  nem derívelhető automatikusan.
        //    -> vagy kihagyod, vagy custom logikát írsz.
        // self.triggers.hash(state);   // ha Trigger is #derive(Hash)
        // self.activated_abilities.hash(state); // ha ActivatedAbility is #derive(Hash)

        // attributes : Vec<Box<dyn CardAttribute>>
        //   Erre tipikusan NINCS egyszerű derive-hash.
        //   Vagy teljesen kihagyod, vagy mindegyik CardAttribute típushoz
        //   egyedi Hash-implementációt írsz, + dyn-diszpatch.

        // attached_to
        self.attached_to.hash(state);
    }
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        // Elég, ha ezek megegyeznek (attributes/activated_abilities-t NEM hasonlítjuk)
        self.name == other.name
            && self.card_type == other.card_type
            && self.type_flags == other.type_flags
            && self.mana_cost == other.mana_cost
            && self.triggers == other.triggers
        // attributes és activated_abilities kimarad
    }
}

impl Eq for Card {}

impl Card {
    /// Létrehoz egy új kártyát, a `card_type` alapján
    /// automatikusan beállítva a `type_flags` mezőt is.
    pub fn new(name: &str, card_type: CardType, mana_cost: ManaCost) -> Self {
        let mut flags = CardTypeFlags::NONE;
        match &card_type {
            CardType::Creature(_) => {
                flags |= CardTypeFlags::CREATURE;
            }
            CardType::Instant => {
                flags |= CardTypeFlags::INSTANT;
            }
            CardType::Enchantment => {
                flags |= CardTypeFlags::ENCHANTMENT;
            }
            CardType::Land => {
                flags |= CardTypeFlags::LAND;
            }
            CardType::Token => {
                flags |= CardTypeFlags::TOKEN;
            }
            CardType::Planeswalker => {
                flags |= CardTypeFlags::PLANESWALKER;
            }
        }
        Card {
            card_id: 0, // GRE osztja ki
            name: name.into(),
            card_type,
            type_flags: flags,
            mana_cost,
            attributes: Vec::new(),
            triggers: Vec::new(),
            activated_abilities: Vec::new(),
            attached_to: None,
        }
    }
    /// Egy triggert és attribútumot ad a kártyához.
    pub fn with(mut self, trigger: Trigger, attr: impl CardAttribute + 'static) -> Self {
        self.triggers.push(trigger);
        self.attributes.push(Box::new(attr));
        self
    }
    /// Egy activated ability-t is hozzáadhatunk
    pub fn with_activated(mut self, ability: ActivatedAbility) -> Self {
        self.activated_abilities.push(ability);
        self
    }

    /// A token bitflag vagy bármely más bitflag hozzáadása
    pub fn with_added_type(mut self, flag: CardTypeFlags) -> Self {
        self.type_flags |= flag;
        self
    }
    /// A creature powerjét átírja (ha creature)
    pub fn with_power(mut self, new_power: i32) -> Self {
        if let CardType::Creature(ref mut c) = self.card_type {
            c.power = new_power;
        }
        self
    }
    /// A creature toughness-ét átírja (ha creature)
    pub fn with_toughness(mut self, new_toughness: i32) -> Self {
        if let CardType::Creature(ref mut c) = self.card_type {
            c.toughness = new_toughness;
        }
        self
    }

    /// A kártyán lévő attribute-öket/trigger-öket futtatjuk le,
    /// megnézve, illik-e a paraméter `trigger`-re.
    pub fn trigger_by(&mut self, trigger: &Trigger) -> Vec<Effect> {
        self.triggers
            .iter()
            .zip(self.attributes.iter_mut())
            .filter_map(|(t, attr)| {
                if t == trigger {
                    attr.on_trigger(trigger)
                } else {
                    None
                }
            })
            .collect()
    }
    /// Általános kártya-klónozó: bemenet az eredeti Card, plusz opcionális power/toughness
    /// felülírás, plusz bitflag hozzáadás.
    pub fn clone_card(
        original: &Card,
        new_power: Option<i32>,
        new_toughness: Option<i32>,
        added_flags: Option<CardTypeFlags>,
    ) -> Card {
        debug!(
            "clone_card called: original id={}, name='{}', new_power={:?}, new_toughness={:?}, added_flags={:?}",
            original.card_id, original.name, new_power, new_toughness, added_flags
        );

        let mut cloned = original.clone();

        if let CardType::Creature(ref mut cr) = cloned.card_type {
            if let Some(p) = new_power {
                debug!("clone_card: setting new power {} (was {})", p, cr.power);
                cr.power = p;
            }
            if let Some(t) = new_toughness {
                debug!(
                    "clone_card: setting new toughness {} (was {})",
                    t, cr.toughness
                );
                cr.toughness = t;
            }
        }

        if let Some(flags) = added_flags {
            debug!("clone_card: adding flags {:?}", flags);
            cloned.type_flags |= flags;
        }

        // Kiírás a visszaadott másolatról
        let (final_power, final_toughness) = match cloned.card_type {
            CardType::Creature(ref cr) => (cr.power, cr.toughness),
            _ => (0, 0),
        };
        debug!(
            "clone_card returning: cloned id={}, name='{}', power={}, toughness={}, flags={:?}",
            cloned.card_id, cloned.name, final_power, final_toughness, cloned.type_flags
        );

        cloned
    }
    pub fn get_current_power(&self, gre: &Gre) -> i32 {
        // 1) Kiindulunk a base powerből
        let base_power = match &self.card_type {
            CardType::Creature(crea) => crea.power,
            _ => {
                info!(
                    "get_current_power('{}'): nem Creature típus, visszatérünk 0-val.",
                    self.name
                );
                return 0;
            }
        };
        info!(
            "get_current_power('{}'): base power = {}",
            self.name, base_power
        );

        // 2) Ideiglenes (ephemeral) buffok (pl. Felonious Rage +2/+0)
        let ephemeral_power = if let CardType::Creature(crea) = &self.card_type {
            crea.ephemeral_power
        } else {
            0
        };
        info!(
            "get_current_power('{}'): ephemeral power = {}",
            self.name, ephemeral_power
        );

        // 3) +1/+1 countereket (vagy egyéb countereket) is beleszámítjuk
        let plus_one_sum = self.find_plus_one_counters();
        info!(
            "get_current_power('{}'): plus_one_counters => +{}",
            self.name, plus_one_sum
        );

        // 4) Aura-szerű buffok: ha van pl. 'Monster' aura csatolva,
        //    vagy bármilyen enchantment token, ami +X/+Y -t ad
        let aura_buff = self.sum_aura_power_bonuses(gre);
        info!(
            "get_current_power('{}'): aura/other buffs => +{}",
            self.name, aura_buff
        );

        // 5) Összeadjuk az egészet
        let total = base_power + ephemeral_power + plus_one_sum + aura_buff;
        info!(
            "get_current_power('{}'): total power = {}",
            self.name, total
        );

        total
    }

    /// Ugyanez toughnessre
    pub fn get_current_toughness(&self, gre: &Gre) -> i32 {
        // 1) Alap toughnesst kiolvassuk
        let base_toughness = match &self.card_type {
            CardType::Creature(crea) => crea.toughness,
            _ => {
                info!(
                    "get_current_toughness('{}'): nem Creature, 0-t adunk vissza.",
                    self.name
                );
                return 0;
            }
        };
        info!(
            "get_current_toughness('{}'): base toughness = {}",
            self.name, base_toughness
        );

        // 2) ephemeral toughness
        let ephemeral_toughness = if let CardType::Creature(crea) = &self.card_type {
            crea.ephemeral_toughness
        } else {
            0
        };
        info!(
            "get_current_toughness('{}'): ephemeral toughness = {}",
            self.name, ephemeral_toughness
        );

        // 3) +1/+1 counterek -> +1 toughness mindegyik
        let plus_one_sum = self.find_plus_one_counters();
        info!(
            "get_current_toughness('{}'): plus_one_counters => +{}",
            self.name, plus_one_sum
        );

        // 4) aura/tárgy buffok
        let aura_buff = self.sum_aura_toughness_bonuses(gre);
        info!(
            "get_current_toughness('{}'): aura/other buffs => +{}",
            self.name, aura_buff
        );

        // 5) Összegezés
        let total = base_toughness + ephemeral_toughness + plus_one_sum + aura_buff;
        info!(
            "get_current_toughness('{}'): total toughness = {}",
            self.name, total
        );

        total
    }
    /// Lekérdezzük az _aktuális_ power/toughness értéket,
    /// figyelembe véve a bázis-statsot, +1/+1 countereket,
    /// aura tokeneket, ephemeral buffokat, stb.
    pub fn current_power_toughness(&self, gre: &Gre) -> (i32, i32) {
        // 1) Ha nem Creature, visszaadjuk (0,0).
        let creature = match &self.card_type {
            CardType::Creature(cr) => cr,
            _ => return (0, 0),
        };

        // Alapértékek
        let mut power = creature.power;
        let mut toughness = creature.toughness;

        // 2) Ha vannak +1/+1 counterek, hozzáadjuk.
        // (Attól függ, hogy a Te kódban a +1/+1 counterek ténylegesen
        //  beleszámolódnak-e a base-powerbe, vagy tárolsz valahol
        //  `plus_one_counter_count: i32` mezőt, stb.)
        // Például:
        let plus_one_count = self.find_plus_one_counters();
        power += plus_one_count;
        toughness += plus_one_count;

        // 3) Ha vannak aura tokenek, amelyek +X/+Y-t és/vagy képességet adnak,
        //    akkor a GRE-ben a "battlefield_creatures" map-ben
        //    megkeressük mindegyik enchantment tokent, amelynek `attached_to == self.card_id`.
        //    Tegyük fel, mind +X/+Y. Akkor:
        for (_cid, possible_aura) in gre.battlefield_creatures.iter() {
            if possible_aura.attached_to == Some(self.card_id) {
                // Ha ez auraszerű token, megnézzük, ad-e buffot
                // mondjuk valamelyik TriggeredEffectAttribute vagy lementett mező alapján
                if let Some((pb, tb)) = possible_aura.get_buff_amount() {
                    power += pb;
                    toughness += tb;
                }
            }
        }

        // 4) Ha vannak "kör végéig" tartó buffok (Felonious Rage: +2/+0 EoT),
        //    azt nem írjuk be sehová, hanem a GRE nálad pl. schedule_delayed-en keresztül
        //    tárolja, vagy ephemeral... Lényeg: bármilyen ponton is van, IDE,
        //    a "számolásba" be kell hozni.
        //    Például, ha van ephemeral Power: 2 a kártyában,
        //    (de a jövőben inkább ne a structban, hanem attribute-ban),
        //    akkor:
        power += creature.ephemeral_power;
        toughness += creature.ephemeral_toughness;

        // ... és még folytathatnánk
        (power, toughness)
    }
    /// Megnézzük, van-e valamilyen +1/+1 counter a kártyán.
    /// Például, ha a Card attribute-jei között szerepel az AddCounterAttribute,
    /// vagy ha van valami “self.num_plus_one_counters” típusú mező, akkor abból összeadjuk.
    pub fn find_plus_one_counters(&self) -> i32 {
        let mut total = 0;

        // 1) Ha van dedikált `num_plus_one_counters` meződ, azt ide beírhatod:
        // total += self.num_plus_one_counters;

        // 2) Ha a countereket a Card attributes‐ben tárolod,
        //    akkor végignézheted a `self.attributes` listát.
        // Példa:
        for attr in &self.attributes {
            // futásidőn megkérdezzük, hogy ez a trait-obj vajon AddCounterAttribute–e
            if let Some(a) = attr.as_any().downcast_ref::<AddCounterAttribute>() {
                if a.counter == CounterType::PlusOnePlusOne {
                    // a.amount a “+1/+1 counter” mennyisége
                    total += a.amount as i32;
                }
            }
            // (plusz hasonló megoldás, ha a FirstTimePerTurnAttribute generált +1/+1–et, stb.)
        }

        total
    }

    /// Ha ez a kártya "Monster" enchantment–token (vagy bármely más aura),
    /// akkor visszaadja, hogy mekkora buffot ad a “gazda” lénynek.
    /// Példa: Monster esetén +1/+1.
    pub fn get_buff_amount(&self) -> Option<(i32, i32)> {
        // Ha Nálad a “Monster” aura neve `'Monster'`, akkor pl.:
        if self.name == "Monster"
            && self.type_flags.contains(CardTypeFlags::ENCHANTMENT)
            && self.type_flags.contains(CardTypeFlags::TOKEN)
        {
            // ez egy aura token, ami +1/+1–et ad
            Some((1, 1))
        } else {
            // Más aura is lehet, pl. "Fairy" +1/+1, "Knight" +2/+1, stb.
            // Vagy ha nem aura, akkor None
            None
        }
    }

    fn sum_aura_power_bonuses(&self, gre: &Gre) -> i32 {
        let mut sum = 0;

        // Végignézzük a GRE-ben, mely enchantment tokenek vannak éppen a battlefielden
        for (_id, aura_card) in &gre.battlefield_creatures {
            // Csak az enchantment token érdekel
            if aura_card.type_flags.contains(CardTypeFlags::ENCHANTMENT)
                && aura_card.type_flags.contains(CardTypeFlags::TOKEN)
            {
                // Ha az aura erre a card_id-ra van attacholva
                if aura_card.attached_to == Some(self.card_id) {
                    // Példa: nézzük meg a neve, ha "Monster"
                    if aura_card.name == "Monster" {
                        // Ez fixen +1/+1
                        sum += 1;
                    }
                    // Vagy keresheted bennük a plusz attribute-öket is
                    // aura_card.attributes, triggers, stb.
                }
            }
        }

        sum
    }

    fn sum_aura_toughness_bonuses(&self, gre: &Gre) -> i32 {
        let mut sum = 0;
        for (_id, aura_card) in &gre.battlefield_creatures {
            if aura_card.type_flags.contains(CardTypeFlags::ENCHANTMENT)
                && aura_card.type_flags.contains(CardTypeFlags::TOKEN)
                && aura_card.attached_to == Some(self.card_id)
            {
                if aura_card.name == "Monster" {
                    sum += 1;
                }
                // Egyéb aura tokenek ...
            }
        }
        sum
    }
}

/// A teljes card library
pub fn build_card_library() -> HashMap<String, Card> {
    let mut lib = HashMap::new();

    // Cacophony Scamp
    lib.insert(
        CACOPHONY_SCAMP.into(),
        Card::new(
            CACOPHONY_SCAMP,
            CardType::Creature(Creature {
                power: 1,
                toughness: 1,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![
                    CreatureType::Phyrexian,
                    CreatureType::Goblin,
                    CreatureType::Warrior,
                ],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnCombatDamage {
                filter: TargetFilter::SelfCard,
            },
            ProliferateAttribute {
                counter: CounterType::PlusOnePlusOne,
                player: PlayerSelector::Controller,
            },
        )
        .with(
            Trigger::OnDeath {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnDeath {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::Damage {
                    amount: Amount::SourcePower,
                    target: TargetFilter::AnyTarget,
                },
            },
        ),
    );

    // Monastery Swiftspear
    lib.insert(
        MONASTERY_SWIFTSPEAR.into(),
        Card::new(
            MONASTERY_SWIFTSPEAR,
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Human, CreatureType::Monk],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnEnterBattlefield {
                filter: TargetFilter::SelfCard,
            },
            GrantAbilityAttribute {
                ability: KeywordAbility::Haste,
                duration: Duration::EndOfTurn,
                target: TargetFilter::SelfCard,
            },
        )
        .with(
            Trigger::OnSpellCast {
                filter: SpellFilter::InstantOrSorcery,
            },
            ProwessAttribute {
                filter: SpellFilter::InstantOrSorcery,
                power: 1,
                toughness: 1,
                duration: Duration::EndOfTurn,
            },
        ),
    );

    // Electrostatic Infantry
    lib.insert(
        ELECTROSTATIC_INFANTRY.into(),
        Card::new(
            ELECTROSTATIC_INFANTRY,
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Dwarf, CreatureType::Wizard],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnEnterBattlefield {
                filter: TargetFilter::SelfCard,
            },
            GrantAbilityAttribute {
                ability: KeywordAbility::Trample,
                duration: Duration::Permanent,
                target: TargetFilter::SelfCard,
            },
        )
        .with(
            Trigger::OnSpellCast {
                filter: SpellFilter::InstantOrSorcery,
            },
            AddCounterAttribute {
                counter: CounterType::PlusOnePlusOne,
                amount: 1,
                target: TargetFilter::SelfCard,
            },
        ),
    );

    // Heartfire Hero
    lib.insert(
        HEARTFIRE_HERO.into(),
        Card::new(
            HEARTFIRE_HERO,
            CardType::Creature(Creature {
                power: 1,
                toughness: 1,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Mouse, CreatureType::Soldier],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnTargetedFirstTimeEachTurn {
                filter: TargetFilter::SelfCard,
            },
            FirstTimePerTurnAttribute {
                base_trigger: Trigger::OnTargetedFirstTimeEachTurn {
                    filter: TargetFilter::SelfCard,
                },
                reset_phase: GamePhase::End,
                action: Effect::AddCounter {
                    counter: CounterType::PlusOnePlusOne,
                    amount: 1,
                    target: TargetFilter::SelfCard,
                },
                used: false,
            },
        ),
    );

    // Screaming Nemesis
    lib.insert(
        "Screaming Nemesis".into(),
        Card::new(
            "Screaming Nemesis",
            CardType::Creature(Creature {
                power: 3,
                toughness: 3,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Haste],
                types: vec![CreatureType::Spirit],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(2, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnEnterBattlefield {
                filter: TargetFilter::SelfCard,
            },
            GrantAbilityAttribute {
                ability: KeywordAbility::Haste,
                duration: Duration::Permanent,
                target: TargetFilter::SelfCard,
            },
        )
        .with(
            Trigger::OnDealtDamage {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnDealtDamage {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::Damage {
                    amount: Amount::SourcePower,
                    target: TargetFilter::AnyTarget,
                },
            },
        )
        .with(
            Trigger::OnDealtDamage {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnDealtDamage {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::PreventLifeGain {
                    player: PlayerSelector::Opponent,
                    duration: Duration::Permanent,
                },
            },
        ),
    );

    // Hired Claw
    lib.insert(
        "Hired Claw".into(),
        Card::new(
            "Hired Claw",
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Lizard, CreatureType::Mercenary],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnAttackWithCreatureType {
                creature_type: CreatureType::Lizard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnAttackWithCreatureType {
                    creature_type: CreatureType::Lizard,
                },
                effect: Effect::Damage {
                    amount: Amount::Fixed(1),
                    target: TargetFilter::OpponentCreature,
                },
            },
        )
        .with_activated(ActivatedAbility {
            cost: ManaCost::new(1, 1, 0, 0, 0, 0),
            condition: Condition::OpponentLostLifeThisTurn,
            effect: Effect::AddCounter {
                counter: CounterType::PlusOnePlusOne,
                amount: 1,
                target: TargetFilter::SelfCard,
            },
            activated_this_turn: false,
        }),
    );

    // -- FONTOS: Manifold Mouse beépített Offspring 2

    lib.insert(
        "Manifold Mouse".into(),
        Card::new(
            "Manifold Mouse",
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Mouse, CreatureType::Soldier],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
        .with_added_type(CardTypeFlags::CREATURE)
        .with(
            Trigger::AtPhase {
                phase: GamePhase::BeginningCombat,
                player: PlayerSelector::Controller,
            },
            ChooseOnConditionAttribute {
                choose: 1,
                options: vec![
                    Effect::GrantAbility {
                        ability: KeywordAbility::DoubleStrike,
                        duration: Duration::EndOfTurn,
                        target: TargetFilter::CreatureType(CreatureType::Mouse),
                    },
                    Effect::GrantAbility {
                        ability: KeywordAbility::Trample,
                        duration: Duration::EndOfTurn,
                        target: TargetFilter::CreatureType(CreatureType::Mouse),
                    },
                ],
            },
        )
        .with(
            Trigger::OnCastResolved,
            ChooseOnConditionAttribute {
                choose: 1,
                options: vec![
                    Effect::Damage {
                        amount: Amount::Fixed(0),
                        target: TargetFilter::AnyTarget,
                    },
                    Effect::Offspring { cost: 2 },
                ],
            },
        ),
    );

    // Slickshot Show-Off
    lib.insert(
        "Slickshot Show-Off".into(),
        Card::new(
            "Slickshot Show-Off",
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Flying, KeywordAbility::Haste],
                types: vec![CreatureType::Bird, CreatureType::Wizard],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnSpellCast {
                filter: SpellFilter::InstantOrSorcery,
            },
            BuffAttribute {
                power: 2,
                toughness: 0,
                duration: Duration::EndOfTurn,
                target: TargetFilter::SelfCard,
            },
        ),
    );

    // Sunset Strikemaster
    lib.insert(
        "Sunset Strikemaster".into(),
        Card::new(
            "Sunset Strikemaster",
            CardType::Creature(Creature {
                power: 3,
                toughness: 1,
                summoning_sickness: true,
                abilities: vec![],
                types: vec![CreatureType::Human, CreatureType::Monk],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
        .with_activated(ActivatedAbility {
            cost: ManaCost::free(),
            condition: Condition::Always,
            effect: Effect::AddMana {
                red: 1,
                colorless: 0,
                blue: 0,
                green: 0,
                black: 0,
                white: 0,
            },
            activated_this_turn: false,
        }),
    );

    // Emberheart Challenger
    lib.insert(
        "Emberheart Challenger".into(),
        Card::new(
            "Emberheart Challenger",
            CardType::Creature(Creature {
                power: 2,
                toughness: 2,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Haste],
                types: vec![CreatureType::Mouse, CreatureType::Warrior],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnSpellCast {
                filter: SpellFilter::InstantOrSorcery,
            },
            ProwessAttribute {
                filter: SpellFilter::InstantOrSorcery,
                power: 1,
                toughness: 1,
                duration: Duration::EndOfTurn,
            },
        )
        .with(
            Trigger::OnTargetedFirstTimeEachTurn {
                filter: TargetFilter::SelfCard,
            },
            FirstTimePerTurnAttribute {
                base_trigger: Trigger::OnTargetedFirstTimeEachTurn {
                    filter: TargetFilter::SelfCard,
                },
                reset_phase: GamePhase::End,
                action: Effect::ExileThenPlayFromExile {
                    count: 1,
                    player: PlayerSelector::Controller,
                    duration: Duration::EndOfTurn,
                },
                used: false,
            },
        ),
    );

    // Felonious Rage
    lib.insert(
        FELONIOUS_RAGE.into(),
        Card::new(
            FELONIOUS_RAGE,
            CardType::Instant,
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnCastResolved,
            TriggeredEffectAttribute {
                trigger: Trigger::OnCastResolved,
                effect: Effect::TargetedEffects {
                    sub_effects: vec![
                        // 1) +2/+0 EoT
                        Effect::ModifyStats {
                            power_delta: 2,
                            toughness_delta: 0,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::Creature,
                        },
                        // 2) Grant Haste EoT
                        Effect::GrantAbility {
                            ability: KeywordAbility::Haste,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::Creature,
                        },
                        // 3) Ha meghal a célpont a körben, hozz létre 2/2 nyomozót
                        Effect::WhenTargetDiesThisTurn {
                            effect: Box::new(Effect::CreateCreatureToken {
                                name: "Detective".into(),
                                power: 2,
                                toughness: 2,
                                creature_types: vec![Detective],
                            }),
                        },
                    ],
                },
            },
        ),
    );

    // Monstrous Rage
    lib.insert(
        MONSTROUS_RAGE.into(),
        Card::new(
            MONSTROUS_RAGE,
            CardType::Instant,
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnCastResolved,
            TriggeredEffectAttribute {
                trigger: Trigger::OnCastResolved,
                effect: Effect::TargetedEffects {
                    sub_effects: vec![
                        // (1) Eredeti, mondjuk +2/+0 EoT a célpontra
                        Effect::ModifyStats {
                            power_delta: 2,
                            toughness_delta: 0,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::Creature,
                        },
                        // (2) Létrehozunk egy 'Monster' enchantmentet,
                        //     ami +1/+1‐et és Trample‐t ad
                        //     addig, amíg fennmarad
                        Effect::CreateEnchantmentToken {
                            name: "Monster".into(),
                            power_buff: 1,
                            toughness_buff: 1,
                            ability: KeywordAbility::Trample,
                        },
                    ],
                },
            },
        ),
    );

    // Blazing Crescendo
    lib.insert(
        BLAZING_CRESCENDO.into(),
        Card::new(
            BLAZING_CRESCENDO,
            CardType::Instant,
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnCastResolved,
            BuffAttribute {
                power: 3,
                toughness: 1,
                duration: Duration::EndOfTurn,
                target: TargetFilter::Creature,
            },
        )
        .with(
            Trigger::OnCastResolved,
            ExileAndPlayAttribute {
                count: 1,
                player: PlayerSelector::Controller,
                duration: Duration::NextTurnEnd,
            },
        ),
    );

    // Demonic Ruckus
    lib.insert(
        DEMONIC_RUCKUS.into(),
        Card::new(
            DEMONIC_RUCKUS,
            CardType::Enchantment,
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnCastResolved,
            BuffAttribute {
                power: 1,
                toughness: 1,
                duration: Duration::Permanent,
                target: TargetFilter::ControllerCreature,
            },
        )
        .with(
            Trigger::OnCastResolved,
            GrantAbilityAttribute {
                ability: KeywordAbility::Menace,
                duration: Duration::Permanent,
                target: TargetFilter::ControllerCreature,
            },
        )
        .with(
            Trigger::OnCastResolved,
            GrantAbilityAttribute {
                ability: KeywordAbility::Trample,
                duration: Duration::Permanent,
                target: TargetFilter::ControllerCreature,
            },
        )
        .with(
            Trigger::OnDeath {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnDeath {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::DrawCards {
                    count: 1,
                    player: PlayerSelector::Controller,
                },
            },
        ),
    );

    // Burst Lightning
    lib.insert(
        BURST_LIGHTNING.into(),
        Card::new(
            BURST_LIGHTNING,
            CardType::Instant,
            ManaCost::new(4, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnCastResolved,
            ConditionalAttribute {
                trigger: Trigger::OnCastResolved,
                condition: Condition::SpellWasKicked,
                effect_if_true: Effect::Damage {
                    amount: Amount::Fixed(4),
                    target: TargetFilter::AnyTarget,
                },
                effect_if_false: Some(Effect::Damage {
                    amount: Amount::Fixed(2),
                    target: TargetFilter::AnyTarget,
                }),
            },
        ),
    );

    // Lightning Strike
    lib.insert(
        LIGHTNING_STRIKE.into(),
        Card::new(
            LIGHTNING_STRIKE,
            CardType::Instant,
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnCastResolved,
            TriggeredEffectAttribute {
                trigger: Trigger::OnCastResolved,
                effect: Effect::Damage {
                    amount: Amount::Fixed(3),
                    target: TargetFilter::AnyTarget,
                },
            },
        ),
    );

    // Gene Pollinator
    lib.insert(
        GENE_POLLINATOR.into(),
        Card::new(
            GENE_POLLINATOR,
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Insect, CreatureType::Robot],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(0, 0, 1, 0, 0, 0),
        )
        .with_activated(ActivatedAbility {
            cost: ManaCost::free(),
            condition: Condition::Tap,
            effect: Effect::ChooseSome {
                choose: 1,
                options: vec![
                    Effect::AddMana {
                        colorless: 0,
                        red: 0,
                        blue: 0,
                        green: 1,
                        black: 0,
                        white: 0,
                    },
                    Effect::AddMana {
                        colorless: 0,
                        red: 0,
                        blue: 0,
                        green: 0,
                        black: 1,
                        white: 0,
                    },
                    Effect::AddMana {
                        colorless: 0,
                        red: 0,
                        blue: 1,
                        green: 0,
                        black: 0,
                        white: 0,
                    },
                    Effect::AddMana {
                        colorless: 0,
                        red: 1,
                        blue: 0,
                        green: 0,
                        black: 0,
                        white: 0,
                    },
                    Effect::AddMana {
                        colorless: 0,
                        red: 0,
                        blue: 0,
                        green: 0,
                        black: 0,
                        white: 1,
                    },
                ],
            },
            activated_this_turn: false,
        }),
    );

    // Llanowar Elves
    lib.insert(
        LLANOWAR_ELVES.into(),
        Card::new(
            LLANOWAR_ELVES,
            CardType::Creature(Creature {
                power: 1,
                toughness: 1,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Elf, CreatureType::Druid],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(0, 0, 1, 0, 0, 0),
        )
        .with_activated(ActivatedAbility {
            cost: ManaCost::free(),
            condition: Condition::Tap,
            effect: Effect::AddMana {
                colorless: 0,
                red: 0,
                blue: 0,
                green: 1,
                black: 0,
                white: 0,
            },
            activated_this_turn: false,
        }),
    );

    // Pawpatch Recruit
    lib.insert(
        PAWPATCH_RECRUIT.into(),
        Card::new(
            PAWPATCH_RECRUIT,
            CardType::Creature(Creature {
                power: 2,
                toughness: 1,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Trample],
                types: vec![CreatureType::Rabbit, CreatureType::Warrior],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(0, 0, 1, 0, 0, 0),
        )
        .with(
            Trigger::OnEnterBattlefield {
                filter: TargetFilter::SelfCard,
            },
            OffspringAttribute { additional_cost: 2 },
        )
        .with(
            Trigger::OnTargetedFirstTimeEachTurn {
                filter: TargetFilter::ControllerCreature,
            },
            FirstTimePerTurnAttribute {
                base_trigger: Trigger::OnTargetedFirstTimeEachTurn {
                    filter: TargetFilter::ControllerCreature,
                },
                reset_phase: GamePhase::End,
                action: Effect::AddCounter {
                    counter: CounterType::PlusOnePlusOne,
                    amount: 1,
                    target: TargetFilter::ControllerCreature,
                },
                used: false,
            },
        ),
    );

    // Badgermole Cub
    lib.insert(
        BADGERMOLE_CUB.into(),
        Card::new(
            BADGERMOLE_CUB,
            CardType::Creature(Creature {
                power: 2,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Badger, CreatureType::Mole],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(1, 0, 1, 0, 0, 0),
        )
        .with(
            Trigger::OnEnterBattlefield {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnEnterBattlefield {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::TargetedEffects {
                    sub_effects: vec![
                        Effect::GrantAbility {
                            ability: KeywordAbility::Haste,
                            duration: Duration::Permanent,
                            target: TargetFilter::ControllerLand,
                        },
                        Effect::AddCounter {
                            counter: CounterType::PlusOnePlusOne,
                            amount: 1,
                            target: TargetFilter::ControllerLand,
                        },
                    ],
                },
            },
        )
        .with(
            Trigger::OnAddMana {
                filter: TargetFilter::ControllerCreature,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnAddMana {
                    filter: TargetFilter::ControllerCreature,
                },
                effect: Effect::AddMana {
                    colorless: 0,
                    red: 0,
                    blue: 0,
                    green: 1,
                    black: 0,
                    white: 0,
                },
            },
        ),
    );

    // Frenzied Baloth
    lib.insert(
        FRENZIED_BALOTH.into(),
        Card::new(
            FRENZIED_BALOTH,
            CardType::Creature(Creature {
                power: 3,
                toughness: 2,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Trample, KeywordAbility::Haste],
                types: vec![CreatureType::Beast],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(3, 0, 2, 0, 0, 0),
        ),
    );

    // Hunter's Talent
    lib.insert(
        HUNTERS_TALENT.into(),
        Card::new(
            HUNTERS_TALENT,
            CardType::Enchantment,
            ManaCost::new(1, 0, 1, 0, 0, 0),
        )
        .with(
            Trigger::OnEnterBattlefield {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnEnterBattlefield {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::DamageByTargetPower {
                    source: TargetFilter::ControllerCreature,
                    target: TargetFilter::OpponentCreature,
                },
            },
        )
        .with(
            Trigger::OnAttack {
                filter: TargetFilter::ControllerCreature,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnAttack {
                    filter: TargetFilter::ControllerCreature,
                },
                effect: Effect::TargetedEffects {
                    sub_effects: vec![
                        Effect::ModifyStats {
                            power_delta: 1,
                            toughness_delta: 0,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::Creature,
                        },
                        Effect::GrantAbility {
                            ability: KeywordAbility::Trample,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::Creature,
                        },
                    ],
                },
            },
        )
        .with(
            Trigger::AtPhase {
                phase: GamePhase::End,
                player: PlayerSelector::Controller,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::AtPhase {
                    phase: GamePhase::End,
                    player: PlayerSelector::Controller,
                },
                effect: Effect::Conditional {
                    condition: Condition::HasCreaturePower4OrMore,
                    effect_if_true: Box::new(Effect::DrawCards {
                        count: 1,
                        player: PlayerSelector::Controller,
                    }),
                    effect_if_false: None,
                },
            },
        ),
    );

    // Overprotect
    lib.insert(
        OVERPROTECT.into(),
        Card::new(
            OVERPROTECT,
            CardType::Instant,
            ManaCost::new(1, 0, 1, 0, 0, 0),
        )
        .with(
            Trigger::OnCastResolved,
            BuffAttribute {
                power: 3,
                toughness: 3,
                duration: Duration::EndOfTurn,
                target: TargetFilter::ControllerCreature,
            },
        )
        .with(
            Trigger::OnCastResolved,
            GrantAbilityAttribute {
                ability: KeywordAbility::Trample,
                duration: Duration::EndOfTurn,
                target: TargetFilter::ControllerCreature,
            },
        )
        .with(
            Trigger::OnCastResolved,
            GrantAbilityAttribute {
                ability: KeywordAbility::Hexproof,
                duration: Duration::EndOfTurn,
                target: TargetFilter::ControllerCreature,
            },
        )
        .with(
            Trigger::OnCastResolved,
            GrantAbilityAttribute {
                ability: KeywordAbility::Indestructible,
                duration: Duration::EndOfTurn,
                target: TargetFilter::ControllerCreature,
            },
        ),
    );

    // Scavenging Ooze
    lib.insert(
        SCAVENGING_OOZE.into(),
        Card::new(
            SCAVENGING_OOZE,
            CardType::Creature(Creature {
                power: 2,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Ooze],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(1, 0, 1, 0, 0, 0),
        )
        .with_activated(ActivatedAbility {
            cost: ManaCost::new(0, 0, 1, 0, 0, 0),
            condition: Condition::Always,
            effect: Effect::TargetedEffects {
                sub_effects: vec![
                    Effect::Exile {
                        target: TargetFilter::CardInGraveyard,
                    },
                    Effect::Conditional {
                        condition: Condition::ExiledCardWasCreature,
                        effect_if_true: Box::new(Effect::TargetedEffects {
                            sub_effects: vec![
                                Effect::AddCounter {
                                    counter: CounterType::PlusOnePlusOne,
                                    amount: 1,
                                    target: TargetFilter::SelfCard,
                                },
                                Effect::GainLife {
                                    amount: 1,
                                    player: PlayerSelector::Controller,
                                },
                            ],
                        }),
                        effect_if_false: None,
                    },
                ],
            },
            activated_this_turn: false,
        }),
    );

    // Archdruid's Charm
    lib.insert(
        ARCHDRUIDS_CHARM.into(),
        Card::new(
            ARCHDRUIDS_CHARM,
            CardType::Instant,
            ManaCost::new(0, 0, 3, 0, 0, 0),
        )
        .with(
            Trigger::OnCastResolved,
            ChooseOnConditionAttribute {
                choose: 1,
                options: vec![
                    Effect::DrawCards {
                        count: 1,
                        player: PlayerSelector::Controller,
                    },
                    Effect::TargetedEffects {
                        sub_effects: vec![
                            Effect::AddCounter {
                                counter: CounterType::PlusOnePlusOne,
                                amount: 1,
                                target: TargetFilter::ControllerCreature,
                            },
                            Effect::DamageByTargetPower {
                                source: TargetFilter::ControllerCreature,
                                target: TargetFilter::OpponentCreature,
                            },
                        ],
                    },
                    Effect::Destroy {
                        target: TargetFilter::Artifact,
                    },
                ],
            },
        ),
    );

    // Surrak, Elusive Hunter
    lib.insert(
        SURRAK_ELUSIVE_HUNTER.into(),
        Card::new(
            SURRAK_ELUSIVE_HUNTER,
            CardType::Creature(Creature {
                power: 4,
                toughness: 3,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Trample],
                types: vec![CreatureType::Human, CreatureType::Warrior],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(2, 0, 1, 0, 0, 0),
        )
        .with(
            Trigger::OnTargeted {
                filter: TargetFilter::ControllerCreature,
                player: PlayerSelector::Opponent,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnTargeted {
                    filter: TargetFilter::ControllerCreature,
                    player: PlayerSelector::Opponent,
                },
                effect: Effect::DrawCards {
                    count: 1,
                    player: PlayerSelector::Controller,
                },
            },
        ),
    );

    // Terrasymbiosis
    lib.insert(
        TERRASYMBIOSIS.into(),
        Card::new(
            TERRASYMBIOSIS,
            CardType::Enchantment,
            ManaCost::new(2, 0, 1, 0, 0, 0),
        )
        .with(
            Trigger::OnCounterAdded {
                filter: TargetFilter::ControllerCreature,
            },
            FirstTimePerTurnAttribute {
                base_trigger: Trigger::OnCounterAdded {
                    filter: TargetFilter::ControllerCreature,
                },
                reset_phase: GamePhase::End,
                action: Effect::DrawCardsCounted,
                used: false,
            },
        ),
    );

    // Tyvar, the Pummeler
    lib.insert(
        TYVAR_THE_PUMMELER.into(),
        Card::new(
            TYVAR_THE_PUMMELER,
            CardType::Creature(Creature {
                power: 3,
                toughness: 3,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Elf, CreatureType::Warrior],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(1, 0, 2, 0, 0, 0),
        )
        .with_activated(ActivatedAbility {
            cost: ManaCost::free(),
            condition: Condition::Always,
            effect: Effect::TargetedEffects {
                sub_effects: vec![
                    Effect::TapTarget {
                        target: TargetFilter::ControllerCreature,
                    },
                    Effect::GrantAbility {
                        ability: KeywordAbility::Indestructible,
                        duration: Duration::EndOfTurn,
                        target: TargetFilter::SelfCard,
                    },
                    Effect::TapTarget {
                        target: TargetFilter::SelfCard,
                    },
                ],
            },
            activated_this_turn: false,
        })
        .with_activated(ActivatedAbility {
            cost: ManaCost::new(3, 0, 2, 0, 0, 0),
            condition: Condition::Always,
            effect: Effect::BuffAllByMaxPower {
                filter: TargetFilter::ControllerCreature,
                duration: Duration::EndOfTurn,
            },
            activated_this_turn: false,
        }),
    );

    // Ouroboroid
    lib.insert(
        OUROBOROID.into(),
        Card::new(
            OUROBOROID,
            CardType::Creature(Creature {
                power: 1,
                toughness: 3,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Plant, CreatureType::Wurm],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(2, 0, 2, 0, 0, 0),
        )
        .with(
            Trigger::AtPhase {
                phase: GamePhase::Combat,
                player: PlayerSelector::Controller,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::AtPhase {
                    phase: GamePhase::Combat,
                    player: PlayerSelector::Controller,
                },
                effect: Effect::AddCounterAll {
                    counter: CounterType::PlusOnePlusOne,
                    amount: Amount::SourcePower,
                    filter: TargetFilter::ControllerCreature,
                },
            },
        ),
    );

    // Agonasaur Rex
    lib.insert(
        AGONASAUR_REX.into(),
        Card::new(
            AGONASAUR_REX,
            CardType::Creature(Creature {
                power: 8,
                toughness: 8,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Trample],
                types: vec![CreatureType::Dinosaur],
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::new(3, 0, 2, 0, 0, 0),
        )
        .with(
            Trigger::OnCycle {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnCycle {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::TargetedEffects {
                    sub_effects: vec![
                        Effect::AddCounter {
                            counter: CounterType::PlusOnePlusOne,
                            amount: 2,
                            target: TargetFilter::Creature,
                        },
                        Effect::GrantAbility {
                            ability: KeywordAbility::Trample,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::Creature,
                        },
                        Effect::GrantAbility {
                            ability: KeywordAbility::Indestructible,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::Creature,
                        },
                    ],
                },
            },
        ),
    );

    // Vivien Reid
    lib.insert(
        VIVIEN_REID.into(),
        Card::new(
            VIVIEN_REID,
            CardType::Planeswalker,
            ManaCost::new(3, 0, 2, 0, 0, 0),
        )
        .with(
            Trigger::OnEnterBattlefield {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnEnterBattlefield {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::AddCounter {
                    counter: CounterType::Loyalty,
                    amount: 5,
                    target: TargetFilter::SelfCard,
                },
            },
        )
        .with_activated(ActivatedAbility {
            cost: ManaCost::free(),
            condition: Condition::Always,
            effect: Effect::DrawCards {
                count: 1,
                player: PlayerSelector::Controller,
            },
            activated_this_turn: false,
        })
        .with_activated(ActivatedAbility {
            cost: ManaCost::free(),
            condition: Condition::Always,
            effect: Effect::Destroy {
                target: TargetFilter::Enchantment,
            },
            activated_this_turn: false,
        }),
    );

    // Ugin, Eye of the Storms
    lib.insert(
        UGIN_EYE_OF_THE_STORMS.into(),
        Card::new(
            UGIN_EYE_OF_THE_STORMS,
            CardType::Planeswalker,
            ManaCost::new(7, 0, 0, 0, 0, 0),
        )
        .with(
            Trigger::OnEnterBattlefield {
                filter: TargetFilter::SelfCard,
            },
            TriggeredEffectAttribute {
                trigger: Trigger::OnEnterBattlefield {
                    filter: TargetFilter::SelfCard,
                },
                effect: Effect::AddCounter {
                    counter: CounterType::Loyalty,
                    amount: 7,
                    target: TargetFilter::SelfCard,
                },
            },
        )
        .with_activated(ActivatedAbility {
            cost: ManaCost::free(),
            condition: Condition::Always,
            effect: Effect::TargetedEffects {
                sub_effects: vec![
                    Effect::GainLife {
                        amount: 3,
                        player: PlayerSelector::Controller,
                    },
                    Effect::DrawCards {
                        count: 1,
                        player: PlayerSelector::Controller,
                    },
                ],
            },
            activated_this_turn: false,
        })
        .with_activated(ActivatedAbility {
            cost: ManaCost::free(),
            condition: Condition::Always,
            effect: Effect::AddMana {
                colorless: 3,
                red: 0,
                blue: 0,
                green: 0,
                black: 0,
                white: 0,
            },
            activated_this_turn: false,
        }),
    );

    // Ba Sing Se
    lib.insert(
        BA_SING_SE.into(),
        Card::new(BA_SING_SE, CardType::Land, ManaCost::free())
            .with_activated(ActivatedAbility {
                cost: ManaCost::free(),
                condition: Condition::Always,
                effect: Effect::AddMana {
                    colorless: 0,
                    red: 0,
                    blue: 0,
                    green: 1,
                    black: 0,
                    white: 0,
                },
                activated_this_turn: false,
            })
            .with_activated(ActivatedAbility {
                cost: ManaCost::new(2, 0, 1, 0, 0, 0),
                condition: Condition::Tap,
                effect: Effect::TargetedEffects {
                    sub_effects: vec![
                        Effect::GrantAbility {
                            ability: KeywordAbility::Haste,
                            duration: Duration::Permanent,
                            target: TargetFilter::ControllerLand,
                        },
                        Effect::AddCounter {
                            counter: CounterType::PlusOnePlusOne,
                            amount: 2,
                            target: TargetFilter::ControllerLand,
                        },
                    ],
                },
                activated_this_turn: false,
            }),
    );
    lib.insert(
        ROCKFACE_VILLAGE.into(),
        Card::new(ROCKFACE_VILLAGE, CardType::Land, ManaCost::free()),
    );


    // Basic lands
        lib.insert(
            MOUNTAIN.into(),
            Card::new(MOUNTAIN, CardType::Land, ManaCost::free()),
        );

        lib.insert(
            FOREST.into(),
            Card::new(FOREST, CardType::Land, ManaCost::free()),
        );

    lib
}
