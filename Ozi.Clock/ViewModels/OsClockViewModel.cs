using System.Windows.Media;

namespace Ozi.Utilities.ViewModels
{
    public class OsClockViewModel(OsClock osClock)
    {
        public string ClockName { get; set; } = osClock.Caption;

        public string SelectedTimeZoneId { get; set; } = osClock.TimeZoneId;

        public Color ClockColor { get; set; } = (Color)ColorConverter.ConvertFromString(osClock.Color);

        public void UpdateModel(OsClock osClock)
        {
            osClock.Caption = ClockName;
            osClock.TimeZoneId = SelectedTimeZoneId;
            osClock.Color = ClockColor.ToString(); // e.g., "#FF383838"
        }
    }
}