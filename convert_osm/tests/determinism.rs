extern crate abstutil;
extern crate convert_osm;

#[test]
fn convert_twice() {
    let flags = convert_osm::Flags {
        osm: "../data/input/tiny_montlake.osm".to_string(),
        elevation: "../data/input/N47W122.hgt".to_string(),
        traffic_signals: "../data/input/TrafficSignals.shp".to_string(),
        parcels: "../data/seattle_parcels.abst".to_string(),
        output: "".to_string(),
    };

    let map1 = convert_osm::convert(&flags);
    let map2 = convert_osm::convert(&flags);

    if map1 != map2 {
        // TODO tmp files
        abstutil::write_json("map1.json", &map1).unwrap();
        abstutil::write_json("map2.json", &map2).unwrap();
        panic!("map1.json and map2.json differ");
    }
}