using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Windows;
using Ozi.Utilities.Settings;
using Ozi.Utilities.ViewModels;

namespace Ozi.Utilities;

public partial class App
{
    public static AppSettings Settings { get; } = LoadSettings();
    public static List<Clock> Clocks { get; } = [];

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

    private static void LoadClocks()
    {
        foreach (var timeZoneInfo in Settings.ClocksSettings)
        {
            Clocks.Add(new Clock(timeZoneInfo.Value.Label,
                timeZoneInfo.Value.TimeZone,
                timeZoneInfo.Value.Color,
                Clocks.Count,
                timeZoneInfo.Value.IsMain ?? false));

            if (timeZoneInfo.Value.IsMain ?? false)
            {
                Settings.MainClockIndex = Clocks.Count - 1;
                Settings.MainTimeZone = timeZoneInfo.Value.TimeZone;
            }
        }
    }

    private static AppSettings LoadSettings()
    {
        var settingsPath = Path.Combine(AppContext.BaseDirectory, "appsettings.json");
        return JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(settingsPath)) ??
               throw new InvalidOperationException();
    }
}