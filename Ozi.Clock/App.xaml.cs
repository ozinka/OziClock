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
    public static List<OsClock> Clocks { get; } = [];

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
            Clocks.Add(new OsClock(timeZoneInfo.Label,
                timeZoneInfo.TimeZone,
                timeZoneInfo.Color,
                timeZoneInfo.IsMain ?? false));

            if (timeZoneInfo.IsMain ?? false)
            {
                Settings.MainClockIndex = Clocks.Count - 1;
                Settings.MainTimeZone = timeZoneInfo.TimeZone;
            }
        }
    }

    private static AppSettings LoadSettings()
    {
        var settingsPath = Path.Combine(AppContext.BaseDirectory, "appsettings.json");

        if (File.Exists(settingsPath))
        {
            try
            {
                return JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(settingsPath)) ??
                       throw new InvalidOperationException();
            }
            catch (Exception e)
            {
                Console.WriteLine(e.Message);
            }
        }

        return DefaultConfig();
    }

    private static AppSettings DefaultConfig()
    {
        return new AppSettings
        {
            MainWndLeft = 0,
            MainWndTop = 0,
            IsTransparent = false,
            Opacity = 0,
            TopMost = false,
            ShowInTaskBar = false,
            UseSnap = false,
            MainClockIndex = 0,
            MainTimeZone = null,
            ClocksSettings =
            [
                new ClockSettings
                {
                    Label = "s",
                    TimeZone = null,
                    Color = null,
                    IsMain = null
                },
                new ClockSettings
                {
                    Label = null,
                    TimeZone = null,
                    Color = null,
                    IsMain = null
                }
            ]
        };
    }
}