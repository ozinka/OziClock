using System;
using System.IO;
using System.Text.Json;
using System.Windows;

namespace Ozi.Utilities;

public partial class App
{
    public static AppSettings Settings { get; } = LoadSettings();

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        Resources["AppSettings"] = Settings;
    }

    protected override void OnExit(ExitEventArgs e)
    {
        base.OnExit(e);
        Settings.Save();
    }

    private static AppSettings LoadSettings()
    {
        var settingsPath = Path.Combine(AppContext.BaseDirectory, "appsettings.json");
        return JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(settingsPath)) ??
               throw new InvalidOperationException();
    }
}