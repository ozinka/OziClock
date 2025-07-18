﻿using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;


namespace Ozi.Utilities.ViewModels;

public class OsClock
{
    const int MarginLeftH = 0;
    const int MarginLeftM = 34;
    const int MarginLeftCol = 30;
    const int MarginLeftShift = 14;

    private string _caption;
    private string _color;
    private bool _showSeconds = true;
    private TimeZoneInfo _timeZone;

    public string TimeZoneId
    {
        get => _timeZone.Id;
        set => SetTimeZone(value);
    }

    public bool ShowSeconds
    {
        get => _showSeconds;
        set
        {
            _showSeconds = value;
            ApplyShowSeconds(value); // Call your custom function when set
        }
    }

    private void ApplyShowSeconds(bool showSeconds)
    {
        _showSeconds = showSeconds;
        if (ShowSeconds)
        {
            _lbDateS.Visibility = Visibility.Visible;
            _lbDateDelim2.Visibility = Visibility.Visible;
            _lbDateH.Margin = new Thickness(MarginLeftH, 35, 0, 0);
            _lbDateM.Margin = new Thickness(MarginLeftM, 35, 0, 0);
            _lbDateDelim.Margin = new Thickness(MarginLeftCol, 41, 0, 0);
        }
        else
        {
            _lbDateS.Visibility = Visibility.Collapsed;
            _lbDateDelim2.Visibility = Visibility.Collapsed;
            _lbDateH.Margin = new Thickness(MarginLeftH + MarginLeftShift, 35, 0, 0);
            _lbDateM.Margin = new Thickness(MarginLeftM + MarginLeftShift, 35, 0, 0);
            _lbDateDelim.Margin = new Thickness(MarginLeftCol + MarginLeftShift, 41, 0, 0);
        }
    }

    public readonly Grid OsGrid;

    public string Caption
    {
        get => _caption;
        set
        {
            _caption = value;
            _lbCapt.Text = value;
        }
    }

    public string Color
    {
        get => _color;
        set
        {
            _color = value;

            // Only apply brushes if UI elements are already created
            if (OsGrid != null)
            {
                var br = new LinearGradientBrush
                {
                    StartPoint = new Point(0.5, -0.05),
                    EndPoint = new Point(0.5, 1)
                };
                br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"), 0.453));
                br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString(_color), 0.776));
                OsGrid.Background = br;
            }

            if (RulerGrid?.Children.Count > 0 && RulerGrid.Children[0] is Canvas canvas)
            {
                var linearGradient = new LinearGradientBrush
                {
                    StartPoint = new Point(0.5, 0),
                    EndPoint = new Point(0.5, 1)
                };
                linearGradient.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString(_color),
                    0.959));
                linearGradient.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"),
                    1.0));

                canvas.Background = linearGradient;
            }
        }
    }

    private void SetTimeZone(string timeZoneId)
    {
        try
        {
            _timeZone = TimeZoneInfo.FindSystemTimeZoneById(timeZoneId);
        }
        catch (TimeZoneNotFoundException)
        {
            _timeZone = TimeZoneInfo.Utc;
        }
    }

    public Grid RulerGrid { get; set; }

    private readonly TextBlock _lbCapt;
    private readonly TextBlock _lbDateMm;
    private readonly TextBlock _lbDateDd;
    private readonly TextBlock _lbDateH;
    private readonly TextBlock _lbDateM;
    private readonly TextBlock _lbDateS;
    private readonly TextBlock _lbDateDelim2;
    private readonly TextBlock _lbDateDelim;
    private bool _isMain;

    public bool IsMain
    {
        get => _isMain;
        set
        {
            _isMain = value;

            if (value)
            {
                _lbCapt.FontWeight = FontWeights.DemiBold;
                _lbCapt.Foreground = new SolidColorBrush(Colors.White);
            }
            else
            {
                _lbCapt.FontWeight = FontWeights.Regular;
                _lbCapt.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9"));
            }
        }
    }

    //constructor
    public OsClock(string caption, string timeZoneId, string color, bool isMain, bool showSeconds = true)
    {
        OsGrid = new Grid
        {
            Width = 99,
            Height = 60,
            HorizontalAlignment = HorizontalAlignment.Left,
            Margin = new Thickness(0, 0, 1, 0)
        };

        var br = new LinearGradientBrush
        {
            StartPoint = new Point(0.5, -0.05),
            EndPoint = new Point(0.5, 1)
        };
        br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"), 0.453));
        br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString(color), 0.776));
        OsGrid.Background = br;
        OsGrid.VerticalAlignment = VerticalAlignment.Bottom;

        TimeZoneId = timeZoneId;

        var fntClcCaption = new FontFamily("Calibry");

        _lbCapt = new TextBlock
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Text = caption,
            Margin = new Thickness(4, 5, 0, 0),
            Width = 48,
            TextTrimming = TextTrimming.None,
            VerticalAlignment = VerticalAlignment.Top,
            HorizontalAlignment = HorizontalAlignment.Left,
            TextAlignment = TextAlignment.Left,
            Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9")),
            // Background = Brushes.White,
        };
        OsGrid.Children.Add(_lbCapt);

        _lbDateMm = new TextBlock
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Text = "11/",
            Margin = new Thickness(51, 5, 0, 0),
            Width = 28,
            TextTrimming = TextTrimming.None,
            VerticalAlignment = VerticalAlignment.Top,
            HorizontalAlignment = HorizontalAlignment.Left,
            TextAlignment = TextAlignment.Right,
            Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9")),
        };
        OsGrid.Children.Add(_lbDateMm);

        _lbDateDd = new TextBlock
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Text = "11",
            Margin = new Thickness(80, 5, 0, 0),
            Width = 18,
            TextTrimming = TextTrimming.None,
            VerticalAlignment = VerticalAlignment.Top,
            HorizontalAlignment = HorizontalAlignment.Left,
            TextAlignment = TextAlignment.Left,
            Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9")),
        };
        OsGrid.Children.Add(_lbDateDd);

        _lbDateDelim = new TextBlock
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Text = ":",
            Margin = new Thickness(showSeconds ? MarginLeftCol : MarginLeftCol + MarginLeftShift, 41, 0, 0),
            Width = 9,
            TextTrimming = TextTrimming.None,
            VerticalAlignment = VerticalAlignment.Top,
            HorizontalAlignment = HorizontalAlignment.Left,
            TextAlignment = TextAlignment.Center,
        };
        OsGrid.Children.Add(_lbDateDelim);

        _lbDateDelim2 = new TextBlock
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Text = ":",
            Margin = new Thickness(67, 41, 0, 0),
            Visibility = showSeconds ? Visibility.Visible : Visibility.Collapsed,
            Width = 9,
            TextTrimming = TextTrimming.None,
            VerticalAlignment = VerticalAlignment.Top,
            HorizontalAlignment = HorizontalAlignment.Left,
            TextAlignment = TextAlignment.Center,
        };
        OsGrid.Children.Add(_lbDateDelim2);

        _lbDateH = new TextBlock
        {
            FontFamily = fntClcCaption,
            FontSize = 22,
            FontWeight = FontWeights.DemiBold,
            Text = "00",
            Margin = new Thickness(showSeconds ? MarginLeftH : MarginLeftH + MarginLeftShift, 35, 0, 0),
            Width = 28,
            TextTrimming = TextTrimming.None,
            VerticalAlignment = VerticalAlignment.Top,
            HorizontalAlignment = HorizontalAlignment.Left,
            TextAlignment = TextAlignment.Right,
        };
        OsGrid.Children.Add(_lbDateH);

        _lbDateM = new TextBlock
        {
            FontFamily = fntClcCaption,
            FontSize = 22,
            Text = "00",
            Margin = new Thickness(showSeconds ? MarginLeftM : MarginLeftM + MarginLeftShift, 35, 0, 0),
            Width = 38,
            TextTrimming = TextTrimming.None,
            VerticalAlignment = VerticalAlignment.Top,
            HorizontalAlignment = HorizontalAlignment.Left,
            TextAlignment = TextAlignment.Center,
        };
        OsGrid.Children.Add(_lbDateM);

        _lbDateS = new TextBlock
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            Text = "00",
            Margin = new Thickness(67, 41, 0, 0),
            Width = 38,
            Visibility = showSeconds ? Visibility.Visible : Visibility.Collapsed,
            TextTrimming = TextTrimming.None,
            TextAlignment = TextAlignment.Center,
        };
        OsGrid.Children.Add(_lbDateS);

        IsMain = isMain;
        Caption = caption;
        _color = color;

        RulerGrid = new Grid
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            Height = 463,
            Margin = new Thickness(0, 0, 1, 0),
            Width = 99
        };
        var canvas = new Canvas
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            UseLayoutRounding = true
        };
        var linearGradient = new LinearGradientBrush
        {
            StartPoint = new Point(0.5, 0),
            EndPoint = new Point(0.5, 1)
        };
        linearGradient.GradientStops.Add(
            new GradientStop((Color)ColorConverter.ConvertFromString(_color), 0.959));
        linearGradient.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"), 1.0));

        canvas.Background = linearGradient;

        Color = _color;

        RulerGrid.Children.Add(canvas);
    }

    public void SetTime(DateTime curTime)
    {
        // Force the Kind to be UTC regardless of what it currently is
        var utcTime = DateTime.SpecifyKind(curTime, DateTimeKind.Utc);

        var tmzTime = TimeZoneInfo.ConvertTimeFromUtc(utcTime, _timeZone);

        _lbDateMm.Text = tmzTime.ToString("MM'/'");
        _lbDateDd.Text = tmzTime.ToString("dd");
        _lbDateH.Text = tmzTime.ToString("HH");
        _lbDateM.Text = tmzTime.ToString("mm");
        _lbDateS.Text = tmzTime.ToString("ss");
    }
}