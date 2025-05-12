using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Media;

namespace Ozi.Utilities.ViewModels
{
    public class OsClockViewModel : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged;

        private string _clockName;
        public string ClockName
        {
            get => _clockName;
            set => SetField(ref _clockName, value);
        }

        private string _selectedTimeZoneId;
        public string SelectedTimeZoneId
        {
            get => _selectedTimeZoneId;
            set => SetField(ref _selectedTimeZoneId, value);
        }

        private Color _clockColor;
        public Color ClockColor
        {
            get => _clockColor;
            set => SetField(ref _clockColor, value);
        }

        private void OnPropertyChanged([CallerMemberName] string? name = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
        }

        private bool SetField<T>(ref T field, T value, [CallerMemberName] string? name = null)
        {
            if (Equals(field, value)) return false;
            field = value;
            OnPropertyChanged(name);
            return true;
        }

        public OsClockViewModel(Clock clock)
        {
            ClockName = clock.Caption;
            SelectedTimeZoneId = clock.TimeZoneId;
            ClockColor = (Color)ColorConverter.ConvertFromString(clock.Color); // conversion here
        }

        public void UpdateModel(Clock clock)
        {
            clock.Caption = ClockName;
            clock.TimeZoneId = SelectedTimeZoneId;
            clock.Color = ClockColor.ToString(); // e.g., "#FF383838"
        }

    }
}