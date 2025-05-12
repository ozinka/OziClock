using System;
using System.Collections.ObjectModel;

namespace Ozi.Utilities.Common;

public static class CommonTimeZones
{
    // Alternative for you BRO, this avoids hardcoding
    public static readonly ReadOnlyCollection<TimeZoneInfo> TimeZones2 = TimeZoneInfo.GetSystemTimeZones();
    
    public static void ShowTimeZones()
    {
        foreach (var tz in TimeZones2)
        {
            Console.WriteLine($"{tz.Id} | {tz.DisplayName} | {tz.BaseUtcOffset} | {tz.StandardName}");
        }
    }
}
            // Output like
            // Pacific Standard Time | (UTC-08:00) Pacific Time (US & Canada) | -08:00:00
            // US Mountain Standard Time | (UTC-07:00) Arizona | -07:00:00
            // Mountain Standard Time (Mexico) | (UTC-07:00) La Paz, Mazatlan | -07:00:00
            // Mountain Standard Time | (UTC-07:00) Mountain Time (US & Canada) | -07:00:00