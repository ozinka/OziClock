namespace Ozi.Utilities.Settings;

public class ClockSettings
{
    public required string Label { get; set; }
    public required string TimeZone { get; set; }
    public required string Color { get; set; }
    public bool? IsMain { get; set; }
}