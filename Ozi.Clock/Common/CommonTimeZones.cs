using System;
using System.Collections.ObjectModel;

namespace Ozi.Utilities.Common;

public static class CommonTimeZones
{
    // Alternative for you BRO, this avoids hardcoding
    public static readonly ReadOnlyCollection<TimeZoneInfo> TimeZones2 = TimeZoneInfo.GetSystemTimeZones();
}