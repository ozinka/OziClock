using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Windows;

namespace Ozi.Utilities;

public partial class App
{
    public static AppSettings Settings { get; } = LoadSettings();
    public static List<OsClock> LstClock { get; } = [];

    private static void LoadClocks()
    {
        foreach (var timeZoneInfo in Settings.TimeZones)
        {
            LstClock.Add(new OsClock(timeZoneInfo.Value.Label ?? timeZoneInfo.Key,
                timeZoneInfo.Value.TimeZone,
                timeZoneInfo.Value.Color,
                LstClock.Count,
                timeZoneInfo.Value.IsMain ?? false));

            if (timeZoneInfo.Value.IsMain ?? false)
            {
                Settings.MainClockIndex = LstClock.Count - 1;
                Settings.MainTimeZone = timeZoneInfo.Value.TimeZone;
            }
        }
    }

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        Resources["AppSettings"] = Settings;
        LoadClocks();
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