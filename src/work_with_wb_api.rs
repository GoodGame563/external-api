use std::time::Duration;
use crate::structure::parse_structures_v1::Root;
use crate::structure::parse_structures_v2::Product as Product_v2;
use crate::structure::parse_structures_v1::Product as Product_v1;
const MAX_RETRIES:u8 = 5;
const RETRY_DELAY: Duration = Duration::from_millis(500);
use rustc_hash::{FxHashMap, FxHashSet};


async fn fetch_url(word: &str) -> Result<String, reqwest::Error> {
    let url = format!("https://search.wb.ru/exactmatch/ru/common/v9/search?ab_daily_autotest=test_group2&appType=1&curr=rub&dest=-2133466&lang=ru&resultset=catalog&sort=popular&spp=30&suppressSpellcheck=false&query={}&page=1", word);
    reqwest::get(&url).await?.text().await
}

async fn fetch_url_by_product(id: u32) -> Result<String, reqwest::Error> {
    let url = format!("https://card.wb.ru/cards/v2/detail?appType=1&curr=rub&dest=-2133466&spp=30&ab_testing=false&lang=ru&nm={}", id);
    reqwest::get(&url).await?.text().await
}

pub async fn get_root_from_url(word: &str) -> Result<Root, String>{
    let mut retry_count = 0;
    loop {
        match fetch_url(word).await{
            Ok(body) => return serde_json::from_str::<Root>(&body).map_err(|e| {
                format!("Not serialization: {}", e)
            }),
            Err(_e) if retry_count < MAX_RETRIES => {
                retry_count += 1; 
                tokio::time::sleep(RETRY_DELAY).await;
            },
            Err(e) => {
                return Err(format!("Get page failed: {}", e))
            },
        }
    }
}

pub async fn get_product_from_url(id: u32) -> Result<Product_v2, String>{
    let mut retry_count = 0;
    loop {
        match fetch_url_by_product(id).await{
            Ok(body) => return Product_v2::from_json(&body).map_err(|e| {
                format!("Not serialization: {}", e)
            }),
            Err(_e) if retry_count < MAX_RETRIES => {
                retry_count += 1; 
                tokio::time::sleep(RETRY_DELAY).await;
            },
            Err(e) => {
                return Err(format!("Get page failed: {}", e))
            },
        }
    }
}

pub fn get_top_10_ids(roots: &[Root]) -> Vec<u64> {
    let mut frequency_map = FxHashMap::with_capacity_and_hasher(
        roots.len() * 10, 
        rustc_hash::FxBuildHasher
        );

    for root in roots {
        for product in &root.data.products {
            *frequency_map.entry(product.id).or_insert(0) += 1;
        }
    }

    let mut counts = Vec::with_capacity(frequency_map.len());
    counts.extend(frequency_map.into_iter().map(|(id, count)| (count, id)));

    if counts.len() <= 10 {
        counts.sort_unstable_by(|a, b| b.cmp(a));
    } else {
        counts.select_nth_unstable_by(9, |a, b| b.cmp(a));
        counts.truncate(10);
        counts.sort_unstable_by(|a, b| b.cmp(a));
    }

    counts.into_iter().map(|(_, id)| id).collect()
}

pub fn get_top_10_ids_with_products(roots: &[Root]) -> Vec<(u64, &Product_v1)> {
    let mut frequency_map = FxHashMap::default();
    let mut product_map = FxHashMap::default();

    for root in roots {
        for product in &root.data.products {
            frequency_map.entry(product.id)
                .and_modify(|c| *c += 1)
                .or_insert(1);
            
            product_map.entry(product.id)
                .or_insert(product);
        }
    }

    let mut counts: Vec<_> = frequency_map.into_iter().collect();
    counts.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    counts.truncate(10);

    counts.into_iter()
        .filter_map(|(id, _)| product_map.get(&id).map(|p| (id, *p)))
        .collect()
}