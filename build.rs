fn main() {
    if let Err(error) = slint_build::compile("ui/app-window.slint") {
        println!("cargo:warning=failed to compile Slint UI: {error}");
        panic!("failed to compile Slint UI: {error}");
    }

    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_manifest(
            r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="0.1.0.0" processorArchitecture="*" name="NeoNote" type="win32"/>
  <description>NeoNote</description>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
    </windowsSettings>
  </application>
</assembly>
"#,
        );

        if let Err(error) = resource.compile() {
            println!("cargo:warning=failed to compile Windows resources: {error}");
        }
    }
}
