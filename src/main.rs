mod yandex;
use yandex::execute_with_yandex;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let data = execute_with_yandex().await?;
    println!("{:#?}", data);
    Ok(())
}