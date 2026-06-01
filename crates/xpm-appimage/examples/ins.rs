use xpm_appimage::AppImageBackend;
fn main(){ let b=AppImageBackend::new().unwrap();
 // clear then install fresh
 for e in b.list_entries(){ let _=b.remove_app(&e.name, &|_:&str|{}); }
 let e=b.install_from_github("AppImage/appimagetool", &|s:&str|print!("{}",s)).unwrap();
 println!("\ninstalled name={} github={:?}", e.name, e.github);
 println!("manifest now has {} entries", b.list_entries().len());
}
