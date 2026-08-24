using System;
using System.Collections.ObjectModel;

namespace Ozi.Utilities.Helpers;

public class TimeZonesHelper
{
    public static readonly ReadOnlyCollection<TimeZoneInfo> SystemTimeZones = TimeZoneInfo.GetSystemTimeZones();
}