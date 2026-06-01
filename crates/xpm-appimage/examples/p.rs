fn main(){ println!("store_dir = {:?}", xpm_appimage::manifest::store_dir());
 println!("manifest = {:?}", xpm_appimage::manifest::manifest_path());
 println!("entries = {}", xpm_appimage::manifest::load().len()); }
