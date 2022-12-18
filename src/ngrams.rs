use std::collections::HashMap;

use crate::*;
use analysis::*;
use findcode::*;
// use rayon::prelude::*;

// use itertools::Itertools;

// struct Ngram {
//     instructions: Vec<MyInstruction>,
// }

// struct NgramSummary {
//     instr_ids: Vec<rabbitizer::InstrId>,
//     count: u32,
// }

fn instr_list(rom_bytes: &[u8], region: &RomRegion) -> Vec<MyInstruction> {
    let region_bytes = &rom_bytes[region.rom_start()..region.rom_end()];
    region_bytes
        .chunks_exact(INSTRUCTION_SIZE)
        .map(|v| MyInstruction::new(read_be_word(v)))
        .collect()
}

fn summary(instrs: &[MyInstruction], n: usize) -> HashMap<Vec<rabbitizer::InstrId>, usize> {
    // let mut ngram_vec: Vec<Vec<rabbitizer::InstrId>> = Vec::with_capacity(instrs.len());
    // Go up to 4 to start with, maybe consider 5 later
    // for window in instrs.par_windows(i) {
    //     ngram_vec.push(
    //         window
    //         .iter()
    //         .map(|x| x.0.instr_id())
    //         .collect::<Vec<rabbitizer::InstrId>>()
    //     );
    // }
    instrs
        .windows(n)
        .map(|w| {
            w.iter()
                .map(|x| x.0.instr_id())
                .collect::<Vec<rabbitizer::InstrId>>()
        })
        .into_iter()
        .fold(HashMap::new(), |mut map, val| {
            map.entry(val).and_modify(|frq| *frq += 1).or_insert(1);
            map
        })
    // instrs
    //     .windows(i)
    //     .map(|w| {
    //         w.iter()
    //             .map(|x| x.0.instr_id())
    //             .collect::<Vec<rabbitizer::InstrId>>()
    //     })
    //     .into_iter()
    //     .counts()
    // Itertools::counts(ngram_vec.into_iter());

    // instrs.windows(i).map(|w| w.iter().map(|x| x.instr_id()).collect()).collect()

    // Vec::new()
}

pub fn print_summary(rom_bytes: &[u8], regions: &[RomRegion], n: usize) {
    let regions_instr_iter = regions
        .iter()
        .map(|r| summary(&instr_list(&rom_bytes, r), n));

    let mut out: HashMap<Vec<rabbitizer::InstrId>, usize> = HashMap::new();
    for map in regions_instr_iter {
        for (k, v) in map {
            out.entry(k).and_modify(|val| *val += v).or_insert(v);
            // let new_v = v + out.get(&k).unwrap_or(&0);
            // out.insert(k, new_v);
        }
    }
    // let t = s
    //     .fold(
    //         || HashMap::new(),
    //         |mut a: HashMap<Vec<rabbitizer::InstrId>, usize>,
    //          b: HashMap<Vec<rabbitizer::InstrId>, usize>| {
    //             a.extend(b.into_iter());
    //             a
    //         },
    //     )
    //     .reduce(
    //         || HashMap::new(),
    //         |mut a, b| {
    //             for (k, v) in b {
    //                 if a.contains_key(k) {
    //                     a.insert(k, v + a[k]);
    //                 } else {
    //                     a.insert(k, v);
    //                 }
    //             }
    //             a
    //         },
    //     );

    // let mut summary = summary(&instr_list(&rom_bytes, &regions), n)
    //     .into_iter()
    //     .collect::<Vec<_>>();

    let mut summary_summary = out.into_iter().collect::<Vec<_>>();
    summary_summary.sort_unstable_by(|x, y| x.1.cmp(&y.1).reverse());

    let mut it = summary_summary.iter();

    println!("{n}-grams for {:?}", regions);
    for _i in 0..10 {
        println!("{:?}", it.next().unwrap());
    }
}
