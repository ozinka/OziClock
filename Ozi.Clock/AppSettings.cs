using System;
using System.Collections.Generic;
using System.ComponentModel; // Required for INotifyPropertyChanged
using System.IO;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Ozi.Utilities
{
    public class AppSettings : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged;

        public List<OsClock>? LstClock = [];

        private double _mainWndLeft;
        public double MainWndLeft
        {
            get => _mainWndLeft;
            set => SetField(ref _mainWndLeft, value);
        }

        private double _mainWndTop;
        public double MainWndTop
        {
            get => _mainWndTop;
            set => SetField(ref _mainWndTop, value);
        }

        private bool _isTransparent;
        public bool IsTransparent
        {
            get => _isTransparent;
            set => SetField(ref _isTransparent, value);
        }

        private double _opacity;
        public double Opacity
        {
            get => _opacity;
            set => SetField(ref _opacity, value);
        }

        private bool _topMost;
        public bool TopMost
        {
            get => _topMost;
            set => SetField(ref _topMost, value);
        }

        private bool _showInTaskBar;
        public bool ShowInTaskBar
        {
            get => _showInTaskBar;
            set => SetField(ref _showInTaskBar, value);
        }

        private bool _useSnap;
        public bool UseSnap
        {
            get => _useSnap;
            set => SetField(ref _useSnap, value);
        }

        public int MainClockIndex = 0;
        public string MainTimeZone = "";

        private Dictionary<string, ClockTimeZoneInfo> _timeZones = new();
        public Dictionary<string, ClockTimeZoneInfo> TimeZones
        {
            get => _timeZones;
            set => SetField(ref _timeZones, value);
        }

        public void Save()
        {
            var options = new JsonSerializerOptions
            {
                WriteIndented = true,
                // ReferenceHandler = ReferenceHandler.Preserve - ? 
            };
            var fileData = JsonSerializer.Serialize(this, options);

            var path = Path.Combine(AppContext.BaseDirectory, "appsettings.json");
            File.WriteAllText(path, fileData);
        }

        protected void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }

        protected bool SetField<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
        {
            if (EqualityComparer<T>.Default.Equals(field, value))
                return false;
            field = value;
            OnPropertyChanged(propertyName);
            return true;
        }
    }
}
