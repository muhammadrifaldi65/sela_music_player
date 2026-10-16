$env:JAVA_HOME = 'D:\android-java\jdk-17.0.20.1+1'
$env:ANDROID_HOME = 'D:\android-sdk'
$env:NDK_HOME = 'D:\android-sdk\ndk\27.2.12479018'
$env:PATH = "$env:JAVA_HOME\bin;$env:ANDROID_HOME\platform-tools;$env:PATH"
npm run tauri -- android build --debug --apk --target aarch64
exit $LASTEXITCODE
