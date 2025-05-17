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
    private static int _mainClockIndex;

    public static string MainTimeZoneId
    {
        get
        {
            int index = MainClockIndex;

            // Check if the Clocks list is not empty and the index is valid
            if (Clocks.Count > 0 && index >= 0 && index < Clocks.Count)
            {
                return Clocks[index].TimeZoneId;
            }

            // Return a default value if there are no clocks or the index is invalid
            return TimeZoneInfo.Local.Id;
        }
    }

    public static int MainClockIndex
    {
        get
        {
            // Return the index of the first clock with isMain = true
            for (int i = 0; i < Clocks.Count; i++)
            {
                if (Clocks[i].IsMain)
                {
                    return i;
                }
            }

            // If no clock is set as main, return default value
            return 0;
        }
        set
        {
            // Set all clocks' isMain property to false except the one at the provided index
            for (int i = 0; i < Clocks.Count; i++)
            {
                Clocks[i].IsMain = (i == value);
            }

            _mainClockIndex = value;
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

    private static void LoadClocks()
    {
        foreach (var timeZoneInfo in Settings.ClocksSettings)
        {
            Clocks.Add(new OsClock(timeZoneInfo.Label,
                timeZoneInfo.TimeZone,
                timeZoneInfo.Color,
                timeZoneInfo.IsMain ?? false));
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