using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;

namespace Ozi.Utilities;

public class AppSettings
{
    public List<OsClock>? LstClock = [];
    public double MainWndLeft { get; set; }
    public double MainWndTop { get; set; }
    public bool IsTransparent { get; set; }
    public double TransparentValue { get; set; }
    public bool TopMost { get; set; }
    public bool ShowInTaskBar { get; set; }
    public bool IsAutoFold { get; set; }
    public bool UseSnap { get; set; }
    public int MainClockIndex = 0;
    public string MainTimeZone = "";
    public Dictionary<string, ClockTimeZoneInfo> TimeZones { get; set; } = new();

    public void Load()
    {
        var settingsPath = Path.Combine(AppContext.BaseDirectory, "appsettings.json");

        if (File.Exists(settingsPath))
        {
            var settings = JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(settingsPath)) ??
                           throw new InvalidOperationException();

            MainWndLeft = settings.MainWndLeft;
            MainWndTop = settings.MainWndTop;
            IsTransparent = settings.IsTransparent;
            TransparentValue = settings.TransparentValue;
            TopMost = settings.TopMost;
            ShowInTaskBar = settings.ShowInTaskBar;
            IsAutoFold = settings.IsAutoFold;
            UseSnap = settings.UseSnap;
            TimeZones = settings.TimeZones;
        }
        else
        {
            throw new FileNotFoundException("appsettings.json not found", settingsPath);
        }
    }

    public void Save()
    {
        var fileData = JsonSerializer.Serialize(this);
        var path = Path.Combine(AppContext.BaseDirectory, "appsettings.json");
        File.WriteAllText(path, fileData);
    }
}