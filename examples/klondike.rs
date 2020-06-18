#![no_std]
#![allow(unused_imports)]

extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};
use anyhow::Error;
use core::mem;
use crankstart::{
    crankstart_game,
    graphics::{
        Bitmap, BitmapDrawMode, BitmapFlip, BitmapTable, Font, Graphics, LCDRect, SolidColor,
    },
    log_to_console, Game, Playdate,
};
use crankstart_sys::{
    PDButtons_kButtonA, PDButtons_kButtonB, PDButtons_kButtonLeft, PDButtons_kButtonRight,
    LCD_COLUMNS, LCD_ROWS,
};
use enum_iterator::IntoEnumIterator;
use euclid::{Point2D, Vector2D};
use hashbrown::HashMap;
use rand::{prelude::*, seq::SliceRandom, SeedableRng};

const SCREEN_CLIP: LCDRect = LCDRect {
    left: 0,
    right: LCD_COLUMNS as i32,
    top: 0,
    bottom: LCD_ROWS as i32,
};

const SCREEN_WIDTH: i32 = LCD_COLUMNS as i32;
//const SCREEN_HEIGHT: i32 = LCD_ROWS as i32;
const MARGIN: i32 = 10;
//const INDEX_MARGIN_X: i32 = 4;
//const INDEX_MARGIN_Y: i32 = 1;
const GUTTER: i32 = 5;
const CARD_WIDTH: i32 = 50;
const CARD_HEIGHT: i32 = 70;

const CRANK_THRESHHOLD: i32 = 10;

#[derive(Clone, Copy, Debug, Eq, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
enum StackId {
    Stock,
    Waste,
    Foundation1,
    Foundation2,
    Foundation3,
    Foundation4,
    Tableau1,
    Tableau2,
    Tableau3,
    Tableau4,
    Tableau5,
    Tableau6,
    Tableau7,
    Hand,
}

impl StackId {
    fn next(&self) -> Self {
        match self {
            StackId::Stock => StackId::Waste,
            StackId::Waste => StackId::Foundation1,
            StackId::Foundation1 => StackId::Foundation2,
            StackId::Foundation2 => StackId::Foundation3,
            StackId::Foundation3 => StackId::Foundation4,
            StackId::Foundation4 => StackId::Tableau1,
            StackId::Tableau1 => StackId::Tableau2,
            StackId::Tableau2 => StackId::Tableau3,
            StackId::Tableau3 => StackId::Tableau4,
            StackId::Tableau4 => StackId::Tableau5,
            StackId::Tableau5 => StackId::Tableau6,
            StackId::Tableau6 => StackId::Tableau7,
            StackId::Tableau7 => StackId::Stock,
            StackId::Hand => StackId::Hand,
        }
    }

    fn previous(&self) -> Self {
        match self {
            StackId::Stock => StackId::Tableau7,
            StackId::Waste => StackId::Stock,
            StackId::Foundation1 => StackId::Waste,
            StackId::Foundation2 => StackId::Foundation1,
            StackId::Foundation3 => StackId::Foundation2,
            StackId::Foundation4 => StackId::Foundation3,
            StackId::Tableau1 => StackId::Foundation4,
            StackId::Tableau2 => StackId::Tableau1,
            StackId::Tableau3 => StackId::Tableau2,
            StackId::Tableau4 => StackId::Tableau3,
            StackId::Tableau5 => StackId::Tableau4,
            StackId::Tableau6 => StackId::Tableau5,
            StackId::Tableau7 => StackId::Tableau6,
            StackId::Hand => StackId::Hand,
        }
    }
}

const FOUNDATIONS: &[StackId] = &[
    StackId::Foundation1,
    StackId::Foundation2,
    StackId::Foundation3,
    StackId::Foundation4,
];

const TABLEAUX: &[StackId] = &[
    StackId::Tableau1,
    StackId::Tableau2,
    StackId::Tableau3,
    StackId::Tableau4,
    StackId::Tableau5,
    StackId::Tableau6,
    StackId::Tableau7,
];

#[derive(Clone, Copy, Debug, Eq, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
enum StackType {
    Stock,
    Waste,
    Foundation,
    Tableau,
    Hand,
}

#[derive(Clone, Copy, Debug, Eq, Hash, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
enum Suit {
    Diamond = 2,
    Club = 1,
    Heart = 3,
    Spade = 4,
}

#[derive(Debug, PartialEq)]
enum Color {
    Black,
    Red,
}

impl Suit {
    fn color(&self) -> Color {
        match self {
            Suit::Diamond | Suit::Heart => Color::Red,
            Suit::Club | Suit::Spade => Color::Black,
        }
    }
}

//const SUITS: &[Suit] = &[Suit::Diamond, Suit::Club, Suit::Heart, Suit::Spade];

#[derive(Clone, Copy, Debug, Eq, Hash, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
enum Rank {
    Ace = 1,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl From<Rank> for &'static str {
    fn from(rank: Rank) -> Self {
        let label = match rank {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "T",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        };
        label
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Card {
    suit: Suit,
    rank: Rank,
    face_up: bool,
}

impl Card {
    fn is_same_color(&self, other: &Card) -> bool {
        self.suit.color() == other.suit.color()
    }

    fn is_one_below(&self, other: &Card) -> bool {
        let delta = other.rank as i32 - self.rank as i32;
        delta == 1
    }
}

pub struct ScreenSpace;
pub type ScreenPoint = Point2D<i32, ScreenSpace>;
pub type ScreenVector = Vector2D<i32, ScreenSpace>;

#[derive(Debug)]
enum FanDirection {
    Down,
    Right,
}

#[derive(Debug)]
enum StackDrawMode {
    Squared,
    Fanned(FanDirection, usize),
}

#[derive(Debug)]
struct Stack {
    stack_id: StackId,
    stack_type: StackType,
    position: ScreenPoint,
    cards: Vec<Card>,
    mode: StackDrawMode,
}

impl Stack {
    pub fn top_card_index(&self) -> usize {
        if self.cards.is_empty() {
            0
        } else {
            self.cards.len() - 1
        }
    }

    pub fn bottom_card(&self) -> Option<&Card> {
        if self.cards.is_empty() {
            None
        } else {
            Some(&self.cards[0])
        }
    }

    pub fn top_card(&self) -> Option<&Card> {
        if self.cards.is_empty() {
            None
        } else {
            Some(&self.cards[self.cards.len() - 1])
        }
    }

    pub fn expose_top_card(&mut self) {
        if !self.cards.is_empty() {
            let last_index = self.cards.len() - 1;
            self.cards[last_index].face_up = true;
        }
    }

    pub fn get_card_position(&self, index: usize) -> ScreenPoint {
        let (vector, count) = match &self.mode {
            StackDrawMode::Squared => (ScreenVector::zero(), 0),
            StackDrawMode::Fanned(direction, visible) => match direction {
                FanDirection::Down => (ScreenVector::new(0, MARGIN), *visible),
                FanDirection::Right => (ScreenVector::new(MARGIN, 0), *visible),
            },
        };
        let number = index.min(count.saturating_sub(1));
        self.position + vector * number as i32
    }

    #[allow(unused)]
    pub fn get_top_card_position(&self) -> ScreenPoint {
        let index = if self.cards.is_empty() {
            0
        } else {
            self.cards.len() - 1
        };
        self.get_card_position(index)
    }

    pub fn previous_active_card(&self, start_index: Option<usize>) -> Option<usize> {
        if self.cards.is_empty() {
            return None;
        }
        let max_index = self.cards.len() - 1;
        let index = if let Some(start_index) = start_index {
            if start_index == 0 {
                return None;
            }
            start_index - 1
        } else {
            max_index
        };
        match self.stack_type {
            StackType::Stock | StackType::Foundation | StackType::Waste => {
                if start_index.is_none() {
                    Some(max_index)
                } else {
                    None
                }
            }
            _ => {
                for active_index in (0..=index).rev() {
                    if self.cards[active_index].face_up {
                        return Some(active_index);
                    }
                }
                None
            }
        }
    }

    pub fn next_active_card(&self, start_index: Option<usize>) -> Option<usize> {
        if self.cards.is_empty() {
            if self.stack_type == StackType::Stock {
                return Some(0);
            }
            return None;
        }
        let max_index = self.cards.len() - 1;
        let index = if let Some(start_index) = start_index {
            start_index + 1
        } else {
            0
        };
        if index <= max_index {
            match self.stack_type {
                StackType::Stock | StackType::Foundation | StackType::Waste => Some(max_index),
                _ => {
                    for active_index in index..=max_index {
                        if self.cards[active_index].face_up {
                            return Some(active_index);
                        }
                    }
                    None
                }
            }
        } else {
            None
        }
    }

    fn foundation_can_accept_hand(&self, hand: &Stack) -> bool {
        if hand.cards.len() > 1 {
            false
        } else {
            if let Some(card) = &hand.top_card() {
                if self.cards.is_empty() {
                    card.rank == Rank::Ace
                } else {
                    if let Some(top_card) = self.top_card() {
                        if card.suit == top_card.suit {
                            top_card.is_one_below(card)
                        } else {
                            false
                        }
                    } else {
                        log_to_console!("foundation has no top card {:?}", self);
                        false
                    }
                }
            } else {
                false
            }
        }
    }

    fn tableau_can_accept_hand(&self, hand: &Stack) -> bool {
        if let Some(card) = &hand.bottom_card() {
            if let Some(top_card) = self.top_card() {
                if !top_card.is_same_color(card) {
                    card.is_one_below(top_card)
                } else {
                    false
                }
            } else {
                card.rank == Rank::King
            }
        } else {
            false
        }
    }

    pub fn can_play(&self, hand: &Stack) -> bool {
        match self.stack_type {
            StackType::Foundation => self.foundation_can_accept_hand(hand),
            StackType::Tableau => self.tableau_can_accept_hand(hand),
            _ => false,
        }
    }

    fn draw_empty(&self, resources: &Resources) -> Result<(), Error> {
        resources.empty.draw(
            None,
            None,
            self.position.x,
            self.position.y,
            BitmapDrawMode::Copy,
            BitmapFlip::Unflipped,
            SCREEN_CLIP,
        )?;
        Ok(())
    }

    fn draw_card_at(
        card: &Card,
        postion: &ScreenPoint,
        resources: &Resources,
    ) -> Result<(), Error> {
        let bitmap = if card.face_up {
            if let Some(bitmap) = resources.card_bitmaps.get(&(card.suit, card.rank)) {
                &bitmap
            } else {
                &resources.empty
            }
        } else {
            &resources.back
        };
        bitmap.draw(
            None,
            None,
            postion.x,
            postion.y,
            BitmapDrawMode::Copy,
            BitmapFlip::Unflipped,
            SCREEN_CLIP,
        )?;
        Ok(())
    }

    fn draw_squared(&self, resources: &Resources) -> Result<(), Error> {
        let card = &self.cards[self.cards.len() - 1];
        let bitmap = if card.face_up {
            resources
                .card_bitmaps
                .get(&(card.suit, card.rank))
                .unwrap_or(&resources.empty)
        } else {
            &resources.back
        };
        bitmap.draw(
            None,
            None,
            self.position.x,
            self.position.y,
            BitmapDrawMode::Copy,
            BitmapFlip::Unflipped,
            SCREEN_CLIP,
        )?;
        Ok(())
    }

    fn draw_fanned(
        &self,
        resources: &Resources,
        source: &Source,
        direction: &FanDirection,
        visible: usize,
    ) -> Result<(), Error> {
        let cards_in_stack = self.cards.len();
        let cards_to_draw = cards_in_stack.min(visible);
        let mut card_pos = self.position;

        let fan_vector = match direction {
            FanDirection::Down => ScreenVector::new(0, MARGIN),
            FanDirection::Right => ScreenVector::new(MARGIN, 0),
        };

        let start = cards_in_stack - cards_to_draw;
        let max_index = cards_in_stack - 1;
        for index in start..cards_in_stack {
            let card = &self.cards[index];
            if card.face_up
                && index < max_index
                && index == source.index
                && self.stack_id == source.stack
            {
                let peeked = card_pos - Vector2D::new(0, CARD_HEIGHT / 4);
                Self::draw_card_at(card, &peeked, resources)?;
            } else {
                Self::draw_card_at(card, &card_pos, resources)?;
            }
            card_pos += fan_vector;
        }

        Ok(())
    }

    fn draw(&self, source: &Source, resources: &Resources) -> Result<(), Error> {
        if self.cards.len() == 0 {
            self.draw_empty(resources)?;
        } else {
            match &self.mode {
                StackDrawMode::Squared => self.draw_squared(resources)?,
                StackDrawMode::Fanned(direction, visible) => {
                    self.draw_fanned(resources, source, direction, *visible)?
                }
            }
        }
        Ok(())
    }

    fn flip_top_card(&mut self) {
        if !self.cards.is_empty() {
            let index = self.cards.len() - 1;
            let card = &mut self.cards[index];
            card.face_up = !card.face_up;
        }
    }
}

fn make_deck() -> Vec<Card> {
    let mut rng = rand_pcg::Pcg32::seed_from_u64(321);

    let mut cards: Vec<Card> = Suit::into_enum_iter()
        .map(move |suit| {
            Rank::into_enum_iter().map(move |rank| Card {
                suit,
                rank,
                face_up: false,
            })
        })
        .flatten()
        .collect();
    cards.shuffle(&mut rng);
    cards
}

struct Resources {
    card_bitmaps: HashMap<(Suit, Rank), Bitmap>,
    back: Bitmap,
    empty: Bitmap,
    #[allow(unused)]
    graphics: Graphics,
    point: Bitmap,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Source {
    stack: StackId,
    index: usize,
}

impl Source {
    fn stock() -> Self {
        Source {
            stack: StackId::Stock,
            index: 0,
        }
    }
}

struct KlondikeGame {
    stock: Stack,
    waste: Stack,
    foundations: Vec<Stack>,
    tableaux: Vec<Stack>,
    source: Source,
    in_hand: Stack,
    target: StackId,
    #[allow(unused)]
    cards_table: BitmapTable,
    resources: Resources,
    crank_threshhold: i32,
}

impl KlondikeGame {
    pub fn load_resources(
        cards_table: &BitmapTable,
        graphics: Graphics,
    ) -> Result<Resources, Error> {
        let mut card_bitmaps = HashMap::new();
        for suit in Suit::into_enum_iter() {
            let row = match suit {
                Suit::Diamond => 2,
                Suit::Heart => 1,
                Suit::Spade => 3,
                Suit::Club => 4,
            };
            let mut col = 0;
            for rank in Rank::into_enum_iter() {
                let index = row * 13 + col;
                let bitmap = cards_table.get_bitmap(index)?;
                card_bitmaps.insert((suit, rank), bitmap);
                col += 1;
            }
        }
        let back = cards_table.get_bitmap(4)?;
        let empty = cards_table.get_bitmap(0)?;
        let point = graphics.load_bitmap("assets/klondike/point")?;
        Ok(Resources {
            card_bitmaps,
            back,
            empty,
            graphics,
            point,
        })
    }

    pub fn new(playdate: &Playdate) -> Result<Box<Self>, Error> {
        let mut cards = make_deck();
        let graphics = playdate.graphics();
        let cards_table = graphics.load_bitmap_table("assets/klondike/cards")?;

        let foundation_gutter_count = (FOUNDATIONS.len() - 1) as i32;
        let mut position = ScreenPoint::new(
            SCREEN_WIDTH
                - FOUNDATIONS.len() as i32 * 50
                - foundation_gutter_count * GUTTER
                - MARGIN,
            MARGIN,
        );

        let foundations: Vec<Stack> = FOUNDATIONS
            .iter()
            .map(|foundation| {
                let stack = Stack {
                    stack_id: *foundation,
                    stack_type: StackType::Foundation,
                    position,
                    cards: Vec::new(),
                    mode: StackDrawMode::Squared,
                };
                position.x += CARD_WIDTH + GUTTER;
                stack
            })
            .collect();

        let mut position = ScreenPoint::new(MARGIN, MARGIN + CARD_HEIGHT + GUTTER);
        let mut stack_count = 1;
        let tableaux: Vec<Stack> = TABLEAUX
            .iter()
            .map(|tableau| {
                let start = cards.len() - stack_count;
                let mut stack = Stack {
                    stack_id: *tableau,
                    stack_type: StackType::Tableau,
                    position,
                    cards: cards.split_off(start),
                    mode: StackDrawMode::Fanned(FanDirection::Down, 52),
                };
                stack.flip_top_card();
                stack_count += 1;
                position.x += 55;
                stack
            })
            .collect();

        let stock = Stack {
            stack_id: StackId::Stock,
            stack_type: StackType::Stock,
            position: ScreenPoint::new(MARGIN, MARGIN),
            cards: cards,
            mode: StackDrawMode::Squared,
        };
        let waste = Stack {
            stack_id: StackId::Waste,
            stack_type: StackType::Waste,
            position: ScreenPoint::new(MARGIN + GUTTER + CARD_WIDTH, MARGIN),
            cards: Vec::new(),
            mode: StackDrawMode::Fanned(FanDirection::Right, 3),
        };
        let in_hand = Stack {
            stack_id: StackId::Hand,
            stack_type: StackType::Hand,
            position: ScreenPoint::zero(),
            cards: Vec::new(),
            mode: StackDrawMode::Squared,
        };
        let resources = Self::load_resources(&cards_table, playdate.graphics())?;
        let source_index = stock.next_active_card(None).unwrap_or(0);
        Ok(Box::new(Self {
            stock,
            waste,
            foundations,
            tableaux,
            source: Source {
                stack: StackId::Stock,
                index: source_index,
            },
            in_hand,
            target: StackId::Stock,
            cards_table,
            resources,
            crank_threshhold: 0,
        }))
    }

    fn get_stack(&self, stack_type: StackId) -> &Stack {
        match stack_type {
            StackId::Stock => &self.stock,
            StackId::Waste => &self.waste,
            StackId::Foundation1 => &self.foundations[0],
            StackId::Foundation2 => &self.foundations[1],
            StackId::Foundation3 => &self.foundations[2],
            StackId::Foundation4 => &self.foundations[3],
            StackId::Tableau1 => &self.tableaux[0],
            StackId::Tableau2 => &self.tableaux[1],
            StackId::Tableau3 => &self.tableaux[2],
            StackId::Tableau4 => &self.tableaux[3],
            StackId::Tableau5 => &self.tableaux[4],
            StackId::Tableau6 => &self.tableaux[5],
            StackId::Tableau7 => &self.tableaux[6],
            StackId::Hand => &self.in_hand,
        }
    }

    fn get_stack_mut(&mut self, stack_type: StackId) -> &mut Stack {
        match stack_type {
            StackId::Stock => &mut self.stock,
            StackId::Waste => &mut self.waste,
            StackId::Foundation1 => &mut self.foundations[0],
            StackId::Foundation2 => &mut self.foundations[1],
            StackId::Foundation3 => &mut self.foundations[2],
            StackId::Foundation4 => &mut self.foundations[3],
            StackId::Tableau1 => &mut self.tableaux[0],
            StackId::Tableau2 => &mut self.tableaux[1],
            StackId::Tableau3 => &mut self.tableaux[2],
            StackId::Tableau4 => &mut self.tableaux[3],
            StackId::Tableau5 => &mut self.tableaux[4],
            StackId::Tableau6 => &mut self.tableaux[5],
            StackId::Tableau7 => &mut self.tableaux[6],
            StackId::Hand => &mut self.in_hand,
        }
    }

    fn cards_in_hand(&self) -> bool {
        self.in_hand.cards.len() > 0
    }

    fn next_active_card(&self) -> Option<Source> {
        let mut source = self.source;
        let mut start = Some(source.index);
        loop {
            let source_stack = self.get_stack(source.stack);
            let next_index = source_stack.next_active_card(start);
            if next_index.is_some() {
                return Some(Source {
                    stack: source.stack,
                    index: next_index.unwrap(),
                });
            } else {
                source.stack = source.stack.next();
                start = None;
            }
        }
    }

    fn previous_active_card(&self) -> Option<Source> {
        let mut source = self.source;
        let mut start = Some(source.index);
        loop {
            let source_stack = self.get_stack(source.stack);
            let previous_index = source_stack.previous_active_card(start);
            if previous_index.is_some() {
                return Some(Source {
                    stack: source.stack,
                    index: previous_index.unwrap(),
                });
            } else {
                source.stack = source.stack.previous();
                start = None;
            }
        }
    }

    fn next_play_location(&self) -> StackId {
        let orginal_stack = self.target;
        let mut target = orginal_stack.next();
        loop {
            let target_stack = self.get_stack(target);
            if target_stack.can_play(&self.in_hand) {
                break;
            } else {
                target = target.next();
            }
            if target == self.source.stack {
                break;
            }
        }
        target
    }

    fn previous_play_location(&self) -> StackId {
        let orginal_stack = self.target;
        let mut target = orginal_stack.previous();
        loop {
            let target_stack = self.get_stack(target);
            if target_stack.can_play(&self.in_hand) {
                break;
            } else {
                target = target.previous();
            }
            if target == self.source.stack {
                break;
            }
        }
        target
    }

    fn deal_from_stock(&mut self) {
        let amount_to_deal = 3.min(self.stock.cards.len());
        if amount_to_deal == 0 {
            mem::swap(&mut self.waste.cards, &mut self.stock.cards);
            for mut card in &mut self.stock.cards {
                card.face_up = false;
            }
            self.stock.cards.reverse();
        } else {
            let start = self.stock.cards.len() - amount_to_deal;
            let mut dealt_cards = self.stock.cards.split_off(start);
            for mut card in &mut dealt_cards {
                card.face_up = true;
            }
            self.waste.cards.append(&mut dealt_cards);
        }
    }

    fn expose_top_card_of_stack(&mut self, stack_id: StackId) {
        let stack = self.get_stack_mut(stack_id);
        stack.expose_top_card();
    }

    fn take_top_card_from_stack(&mut self, stack_id: StackId) {
        let stack = self.get_stack_mut(stack_id);
        let count = stack.cards.len();
        if count > 0 {
            let last_index = count - 1;
            let pt = stack.get_card_position(last_index);
            let mut card = stack.cards.remove(last_index);
            card.face_up = true;
            self.in_hand.cards.push(card);
            self.in_hand.position = pt + Vector2D::new(10, 10);
        }
    }

    fn take_selected_cards_from_stack(&mut self, stack_id: StackId, index: usize) {
        let cards_for_hand = {
            let stack = self.get_stack_mut(stack_id);
            stack.cards.split_off(index)
        };
        let stack = self.get_stack(stack_id);
        let count = cards_for_hand.len();
        if count > 0 {
            let last_index = count - 1;
            let pt = stack.get_card_position(last_index);
            self.in_hand.position = pt + Vector2D::new(10, 10);
            self.in_hand.cards = cards_for_hand;
        }
    }

    fn put_hand_on_target(&mut self) {
        let target = self.target;
        let mut cards = Vec::new();
        mem::swap(&mut cards, &mut self.in_hand.cards);
        let target_stack = self.get_stack_mut(target);
        let index = target_stack.cards.len();
        target_stack.cards.append(&mut cards);
        self.expose_top_card_of_stack(self.source.stack);
        self.source = Source {
            stack: target,
            index: index,
        };
    }

    fn go_next(&mut self) -> Result<(), Error> {
        if self.cards_in_hand() {
            self.target = self.next_play_location();
        } else {
            self.source = self.next_active_card().unwrap_or_else(|| Source::stock())
        }
        Ok(())
    }

    fn go_previous(&mut self) -> Result<(), Error> {
        if self.cards_in_hand() {
            self.target = self.previous_play_location();
        } else {
            self.source = self
                .previous_active_card()
                .unwrap_or_else(|| Source::stock());
        }
        Ok(())
    }

    fn check_crank(&mut self, playdate: &mut Playdate) -> Result<(), Error> {
        let change = playdate.system().get_crank_change()? as i32;
        self.crank_threshhold += change;

        if self.crank_threshhold > CRANK_THRESHHOLD {
            self.go_next()?;
            self.crank_threshhold = -CRANK_THRESHHOLD;
        } else if self.crank_threshhold < -CRANK_THRESHHOLD {
            self.go_previous()?;
            self.crank_threshhold = CRANK_THRESHHOLD;
        }
        Ok(())
    }

    fn check_buttons(&mut self, playdate: &mut Playdate) -> Result<(), Error> {
        let (_, pushed, _) = playdate.system().get_button_state()?;
        if (pushed & PDButtons_kButtonA) != 0 || (pushed & PDButtons_kButtonB) != 0 {
            if self.cards_in_hand() {
                self.put_hand_on_target();
            } else {
                match self.source.stack {
                    StackId::Stock => self.deal_from_stock(),
                    StackId::Waste
                    | StackId::Foundation1
                    | StackId::Foundation2
                    | StackId::Foundation3
                    | StackId::Foundation4 => self.take_top_card_from_stack(self.source.stack),
                    StackId::Tableau1
                    | StackId::Tableau2
                    | StackId::Tableau3
                    | StackId::Tableau4
                    | StackId::Tableau5
                    | StackId::Tableau6
                    | StackId::Tableau7 => {
                        self.take_selected_cards_from_stack(self.source.stack, self.source.index)
                    }
                    StackId::Hand => (),
                }
                self.target = self.source.stack;
            }
        } else if pushed & PDButtons_kButtonLeft != 0 {
            self.go_previous()?;
        } else if pushed & PDButtons_kButtonRight != 0 {
            self.go_next()?;
        }
        Ok(())
    }
}

impl Game for KlondikeGame {
    fn update(
        &mut self,
        playdate: &mut crankstart::Playdate,
    ) -> core::result::Result<(), anyhow::Error> {
        self.check_crank(playdate)?;
        self.check_buttons(playdate)?;

        playdate.graphics().clear(SolidColor::White)?;
        self.stock.draw(&self.source, &self.resources)?;
        self.waste.draw(&self.source, &self.resources)?;
        for stack in &self.foundations {
            stack.draw(&self.source, &self.resources)?;
        }
        for stack in &self.tableaux {
            stack.draw(&self.source, &self.resources)?;
        }

        let cards_in_hand = self.cards_in_hand();

        let position = if cards_in_hand {
            let target = self.get_stack(self.target);
            let position =
                target.get_card_position(target.top_card_index()) + Vector2D::new(10, 10);
            self.in_hand.position = position;
            self.in_hand.draw(&self.source, &self.resources)?;
            position
        } else {
            let source = self.get_stack(self.source.stack);
            source.get_card_position(self.source.index)
        };

        self.resources.point.draw(
            None,
            None,
            position.x + CARD_WIDTH / 2,
            position.y + CARD_HEIGHT / 2,
            BitmapDrawMode::Copy,
            BitmapFlip::Unflipped,
            SCREEN_CLIP,
        )?;

        Ok(())
    }
}

#[cfg(not(test))]
crankstart_game!(KlondikeGame);
