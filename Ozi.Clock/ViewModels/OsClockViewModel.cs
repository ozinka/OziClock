using System.Windows.Media;

namespace Ozi.Utilities.ViewModels
{
    public class OsClockViewModel(Clock clock)
    {
        public string ClockName { get; set; } = clock.Caption;

        public string SelectedTimeZoneId { get; set; } = clock.TimeZoneId;

        public Color ClockColor { get; set; } = (Color)ColorConverter.ConvertFromString(clock.Color);

        public void UpdateModel(Clock clock)
        {
            clock.Caption = ClockName;
            clock.TimeZoneId = SelectedTimeZoneId;
            clock.Color = ClockColor.ToString(); // e.g., "#FF383838"
        }
    }
}