use serde::{Serialize, Deserialize};
use cid::Cid;
use ipfs_unixfs::file::adder::FileAdder;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub name: String,
    pub jersey_number: u8,
    pub description: String,
    pub external_url: String,
    pub image: String,
    pub tier: u8,
    pub overall_rating: f64,
    pub skill_multiplier: f64,
    pub skill: Skill,
    pub attributes: Vec<Attribute>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Skill {
    pub speed: u8,
    pub shooting: u8,
    pub passing: u8,
    pub dribbling: u8,
    pub defense: u8,
    pub physical: u8,
    pub goal_tending: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    pub display_type: String,
    pub trait_type: String,
    pub value: f64,
}

#[derive(Clone, Debug)]
pub struct FileStats {
    pub cid: Vec<u8>,
    pub blocks: usize,
    pub bytes: u64,
}

pub trait ComputeCid: Serialize {
    fn compute_cid(&self) -> FileStats;
    fn cid_string(&self) -> String;
    fn formatted_cid(&self) -> String;
}

impl<T> ComputeCid for T
where
    T: Serialize,
{
    fn compute_cid(&self) -> FileStats {
        // Serialize self into a JSON string
        let json_string = serde_json::to_string(self).unwrap();
        let bytes = json_string.as_bytes();

        // Call the provided compute_cid function with the bytes
        compute_cid(bytes)
    }

    fn cid_string(&self) -> String {
        let cid = self.compute_cid().cid;
        Cid::try_from(cid).unwrap().to_string()
    }

    fn formatted_cid(&self) -> String {
        let cid_string = self.cid_string();
        ["ipfs://", &cid_string].concat()
    }
}

// Provided compute_cid function and FileAdder (assumed to be defined elsewhere)
pub fn compute_cid(input: &[u8]) -> FileStats {
    let mut adder = FileAdder::default();

    for byte in input {
        adder.push(&[*byte]);
    }

    let blocks = adder.finish();
    let mut stats = FileStats {
        cid: Vec::new(),
        blocks: 0,
        bytes: 0,
    };
    for (cid, block) in blocks {
        stats.cid = cid.to_bytes();
        stats.blocks += 1;
        stats.bytes += block.len() as u64;
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cid() {
        let player = Player {
            name: "Lionel Messi".to_string(),
            jersey_number: 10,
            description: "A professional footballer who plays as a forward for Paris Saint-Germain and the Argentina national team.".to_string(),
            external_url: "https://en.wikipedia.org/wiki/Lionel_Messi".to_string(),
            image: "https://upload.wikimedia.org/wikipedia/commons/4/47/Lionel_Messi_20180626.jpg".to_string(),
            tier: 1,
            overall_rating: 94.0,
            skill_multiplier: 1.0,
            skill: Skill {
                speed: 90,
                shooting: 95,
                passing: 90,
                dribbling: 96,
                defense: 32,
                physical: 68,
                goal_tending: 0,
            },
            attributes: vec![
                Attribute {
                    display_type: "Physical".to_string(),
                    trait_type: "Height".to_string(),
                    value: 170.0,
                },
                Attribute {
                    display_type: "Physical".to_string(),
                    trait_type: "Weight".to_string(),
                    value: 72.0,
                },
            ],
        };

        let stats = player.compute_cid();
        println!("{:?}", stats);
        println!("{:02X?}", &player.compute_cid().cid[2..].to_vec());
        // 0xCB8A8DE3C125E9EEE950071D181386F899492E1F8E1ADB5B2D1FEC44BC388050
        // assert_eq!(stats.blocks, 1);
        // assert_eq!(stats.bytes, 1024);
    }
}
