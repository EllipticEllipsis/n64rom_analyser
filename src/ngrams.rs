// use std::collections::HashMap;

use crate::*;
use analysis::*;
use dashmap::DashMap;
use findcode::*;
use rayon::prelude::*;
use std::hash::BuildHasherDefault;
use rustc_hash::FxHasher;

type MyHasher = BuildHasherDefault<FxHasher>;

fn instr_list(rom_bytes: &[u8], region: &RomRegion) -> Vec<MyInstruction> {
    let region_bytes = &rom_bytes[region.rom_start()..region.rom_end()];
    region_bytes
        .chunks_exact(INSTRUCTION_SIZE)
        .map(|v| MyInstruction::new(read_be_word(v)))
        .collect()
}

fn summary(instrs: &[MyInstruction], n: usize) -> DashMap<Vec<rabbitizer::InstrId>, usize, MyHasher> {
    // Could use Itertools::counts, but for now this avoids yet another dependency
    instrs
        .windows(n)
        .map(|w| {
            w.iter()
                .map(|x| x.0.unique_id)
                .collect::<Vec<rabbitizer::InstrId>>()
        })
        .into_iter()
        .fold(DashMap::default(), |map, val| {
            map.entry(val).and_modify(|frq| *frq += 1).or_insert(1);
            map
        })
}

pub fn print_summary(rom_bytes: &[u8], regions: &[RomRegion], n: usize) {
    // No such thing as 0-grams
    assert_ne!(n, 0);

    let out: DashMap<Vec<rabbitizer::InstrId>, usize, MyHasher> = DashMap::default();

    regions.par_iter().for_each(|r| {
        for (k, v) in summary(&instr_list(&rom_bytes, r), n) {
            out.entry(k).and_modify(|val| *val += v).or_insert(v);
        }
    });

    let mut summary_summary = out.into_iter().collect::<Vec<_>>();
    summary_summary.sort_unstable_by(|x, y| x.1.cmp(&y.1).reverse());

    
    let largest = summary_summary.first().unwrap().1;
    let mut it = summary_summary.iter();
    let instruction_count = regions.iter().fold(0, |a, r| a + (r.rom_end() - r.rom_start() ) / 4 );

    println!("{n}-grams for {} regions, {} instructions", regions.len(), instruction_count);
    while let Some(cur) = it.next()  {
        if cur.1 < largest / 5 {
            break;
        }
        println!("{:?}", it.next().unwrap());
    }
}
