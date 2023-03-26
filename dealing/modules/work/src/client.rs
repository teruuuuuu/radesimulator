

use shared_util::cp::CpPrice;



#[derive(Debug)]
struct Listener {

}

fn main() {
    test();
}

fn test() -> Result<(), ()>{
    for n in 1..=100 {
        let ret = match n {
            i if (i < 10) => {
                i
            },
            _ => return Ok(())
        };
        println!("{}", ret);
    }
    Ok(())
}