# Sync Folder CLI

A command-line tool written in Rust for bi-directional file synchronization between a local machine and cloud storage.

## ⚠️ Current Support & Limitations

- **Supported Drives:** Currently, **only Yandex.Disk is supported**. Google Drive selection is not implemented yet.
    
- **Directory Sandbox:** The application operates **strictly** within a sandboxed directory on your cloud drive: `Applications/<your_app_name>/`. It does not have access to any other files or folders on your cloud drive. You can safely manage files inside this sandbox folder via the official Yandex website or mobile app.
    

## ⚙️ Principle of Operation

1. **Local Storage (`static` folder):** Upon execution, the application automatically creates a folder named `static` in the exact same directory where the executable file is located. All synchronized content is stored inside this `static` directory.
    
2. **Delta Calculation:** The application scans the local `static` directory and compares its content with the sandboxed folder in the cloud. It accurately calculates which files are missing locally and which are missing in the cloud.
    
3. **Execution Control:** You can specify the maximum number of concurrent network requests (from 1 to 10) to control the bandwidth load.
    
4. **Sync Modes:** You can choose to download missing files, upload new local files, or run both operations sequentially.
    

## 🔑 How to Get a Yandex OAuth Token

Detailed official documentation is available here: [https://yandex.ru/dev/id/doc/ru/register-api](https://yandex.ru/dev/id/doc/ru/register-api). Follow these steps to obtain your application token:

1. Go to the Yandex OAuth platform: [https://oauth.yandex.ru/client/new/](https://oauth.yandex.ru/client/new/) (log in to your Yandex account if necessary).
    
2. Select **"Для доступа к API или отладке"** (For API access or debugging).
    
3. **App Name:** Enter any name you prefer.
    
    > Note: This name will be used to create the specific folder on your drive: `Applications/<your_app_name>/`.
    
4. **Permissions:** In the **"Доступ к данным"** (Data access) section, select **only** the following permission: `Доступ к папке приложения на диске` (Access to the application folder on the drive).
    
5. Click the submit button to create the application.
    
6. Copy the generated **ClientID**. It is required to receive your actual authorization token.
    
7. Open your web browser and navigate to the following URL, replacing `<ClientID>` with your copied value:
    

Plaintext

```
https://oauth.yandex.ru/authorize?response_type=token&client_id=<ClientID>
```

8. Confirm the access request, copy the displayed token, and save it in a secure place.
    
    > ⚠️ WARNING: The token is displayed **only once** and cannot be viewed later in your personal developer account. If you close the link or lose the token, you will have to repeat the step by navigating to the URL with your ClientID again.
    

## 🚀 Configuration & Execution

You can provide the token to the application using one of the following methods:

### Method 1: Environment File (Recommended)

Create a `.env` file in the same directory where the executable file is located and add your token:


```
#.env
YANDEX_TOKEN=your_oauth_token_here
```

If this file is present, the application automatically reads the token, and you can skip the manual token input prompt by pressing Enter.

### Method 2: Manual Input

Run the application and paste your token directly into the terminal prompt when requested.

### Running the Application 
**1.Compiling:** 
```bash
cargo build --release
```

After a successful build, the executable file is generated in the `target/release/` directory.

**2. Portability:** The executable file (`sync_folder` on Linux or `sync_folder.exe` on Windows) is completely standalone. You can safely copy or move this single file to any directory or location on your system and run it from there. All other files generated inside the `target/release/` folder are only needed during the build process and can be ignored or deleted.

**3. Execution:** Go to the directory where you placed your executable file and run it:

- **On Windows:** Simply double-click the `sync_folder.exe` file, or run it via terminal.
    
- **On Linux:** Open your terminal in the executable's directory and run:

```bash
   ./sync_folder
```

### Interactive Flow Steps

1. **Choose Drive:** Select Yandex.
    
2. **App Token:** Paste your token or press Enter to fall back to the `.env` file.
    
3. **Media Type:** Select the sync target directory (Audio, Video, Image).
    
4. **Concurrency:** Set the limit for concurrent requests (Default: 5, range: 1-10).
    
5. **Sync Mode:** Choose between `All` (Download + Upload), `Download` only, or `Upload` only.